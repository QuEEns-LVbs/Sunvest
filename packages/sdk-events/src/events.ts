import {
  decodeScValBase64,
  scValToNative,
  type ScVal,
  type ScValNative,
} from './xdr.ts';

/**
 * The three event families `src/ecs_events.rs` publishes under the `Sunvest`
 * namespace:
 *
 * | Topics                          | Emitted by                       | Payload              |
 * | ------------------------------- | -------------------------------- | -------------------- |
 * | `("Sunvest", "set", <component>)` | `World::set_typed_observed`      | `entity_id`, `data`  |
 * | `("Sunvest", "del", <component>)` | `World::remove_observed`         | `entity_id`          |
 * | `("Sunvest", "rich", <component>)`| `World::set_rich_observed`       | `entity_id` only     |
 */
export type SunvestEventFamily = 'set' | 'del' | 'rich';

export const Sunvest_NAMESPACE = 'Sunvest';

/**
 * A contract event as returned by Soroban RPC `getEvents` (protocol 23 shape).
 * `topic` entries and `value` are base64-encoded XDR `ScVal` unions.
 */
export interface SorobanRpcContractEvent {
  type: string;
  ledger: number;
  ledgerClosedAt: string;
  contractId?: string;
  id: string;
  pagingToken: string;
  inSuccessfulContractCall: boolean;
  txHash: string;
  topic: string[];
  value: string;
}

export interface SorobanRpcEventsPage {
  events: SorobanRpcContractEvent[];
  latestLedger: number;
  /** Present when the RPC node is willing to resume from this cursor. */
  cursor?: string;
}

export interface SunvestEventBase {
  family: SunvestEventFamily;
  /** `component_type` symbol from the third topic (`"position"`, ...). */
  componentType: string;
  entityId: number;
  ledger: number;
  ledgerClosedAt: string;
  txHash: string;
  pagingToken: string;
  eventId: string;
  contractId?: string;
  /** The untouched RPC event, for consumers that need the raw XDR. */
  raw: SorobanRpcContractEvent;
}

/** `("Sunvest", "set", component)` — carries the serialized component bytes. */
export interface ComponentSetUpdate extends SunvestEventBase {
  family: 'set';
  /**
   * The exact bytes produced by `ComponentTrait::serialize`. This is the same
   * representation the contract stores, so callers can decode it with the
   * component's own field layout.
   */
  data: Uint8Array;
}

/** `("Sunvest", "del", component)` — the component was removed. */
export interface ComponentDeleteUpdate extends SunvestEventBase {
  family: 'del';
}

/**
 * `("Sunvest", "rich", component)` — the component changed but the new value is
 * **not** in the event.
 *
 * Rich components use Soroban's XDR codec and live in instance storage, so the
 * event is a change notification only. `requiresFollowUpRead` is the literal
 * `true` so the compiler forces every consumer to branch on it: a client
 * cannot read `.data` off this record because it does not exist.
 */
export interface RichComponentChangedUpdate extends SunvestEventBase {
  family: 'rich';
  requiresFollowUpRead: true;
  followUp: {
    kind: 'contractData';
    contractId?: string;
    componentType: string;
    entityId: number;
  };
}

export type SunvestEventUpdate =
  | ComponentSetUpdate
  | ComponentDeleteUpdate
  | RichComponentChangedUpdate;

/** True when the event's first two topics are `("Sunvest", set|del|rich)`. */
export function isSunvestEvent(event: SorobanRpcContractEvent): boolean {
  return decodeSunvestTopics(event) !== null;
}

export interface SunvestTopics {
  family: SunvestEventFamily;
  componentType: string;
  /** Decoded third topic, kept for callers that want the raw ScVal. */
  componentTypeScVal: ScVal;
}

/**
 * Decode the `Sunvest` topics of an event, or return `null` when this is not a
 * Sunvest component event. Never throws on non-Sunvest events: a page of RPC
 * events from a shared contract can contain anything.
 */
export function decodeSunvestTopics(event: SorobanRpcContractEvent): SunvestTopics | null {
  const topics = event.topic;
  if (!Array.isArray(topics) || topics.length < 3) return null;
  try {
    const namespace = scValToNative(decodeScValBase64(topics[0] as string));
    if (namespace !== Sunvest_NAMESPACE) return null;
    const family = scValToNative(decodeScValBase64(topics[1] as string));
    if (family !== 'set' && family !== 'del' && family !== 'rich') return null;
    const componentTypeScVal = decodeScValBase64(topics[2] as string);
    const componentTypeNative = scValToNative(componentTypeScVal);
    const componentType =
      typeof componentTypeNative === 'string' ? componentTypeNative : String(componentTypeNative);
    return { family, componentType, componentTypeScVal };
  } catch {
    // Malformed XDR in a shared contract's event stream is not our event.
    return null;
  }
}

interface EventData {
  entityId: number;
  data?: Uint8Array;
}

/**
 * Read `entity_id` (and, for `set`, `data`) out of the event value.
 *
 * `#[contractevent]` encodes the non-topic struct fields as a map keyed by
 * symbol, e.g. `{ entity_id: u32, data: bytes }`. Older/hand-built events may
 * inline a bare `u32`, so both shapes are accepted.
 */
function readEventData(value: ScVal): EventData {
  if (value.type === 'map') {
    const native = scValToNative(value);
    if (typeof native !== 'object' || native === null || Array.isArray(native)) {
      throw new TypeError('Sunvest event value map did not decode to an object');
    }
    const record = native as { [key: string]: ScValNative };
    const entityId = record['entity_id'];
    if (typeof entityId !== 'number') {
      throw new TypeError('Sunvest event is missing a numeric `entity_id` field');
    }
    const data = record['data'];
    if (data === undefined) return { entityId };
    if (!(data instanceof Uint8Array)) {
      throw new TypeError('Sunvest `set` event `data` is not XDR bytes');
    }
    return { entityId, data };
  }
  if (value.type === 'u32') {
    return { entityId: value.value };
  }
  if (value.type === 'u64' || value.type === 'i64') {
    return { entityId: Number(value.value) };
  }
  throw new TypeError(`Unexpected Sunvest event value type: ${value.type}`);
}

/**
 * Decode one RPC event into a typed Sunvest update, or `null` when the event is
 * not one of the three `Sunvest` families.
 */
export function decodeSunvestEvent(event: SorobanRpcContractEvent): SunvestEventUpdate | null {
  const topics = decodeSunvestTopics(event);
  if (topics === null) return null;
  const value = decodeScValBase64(event.value);
  const data = readEventData(value);

  const base: SunvestEventBase = {
    family: topics.family,
    componentType: topics.componentType,
    entityId: data.entityId,
    ledger: event.ledger,
    ledgerClosedAt: event.ledgerClosedAt,
    txHash: event.txHash,
    pagingToken: event.pagingToken,
    eventId: event.id,
    raw: event,
    ...(event.contractId === undefined ? {} : { contractId: event.contractId }),
  };

  if (topics.family === 'set') {
    if (data.data === undefined) {
      throw new TypeError(
        `Sunvest "set" event for ${topics.componentType} did not carry component bytes`,
      );
    }
    return { ...base, family: 'set', data: data.data };
  }

  if (topics.family === 'del') {
    return { ...base, family: 'del' };
  }

  return {
    ...base,
    family: 'rich',
    requiresFollowUpRead: true,
    followUp: {
      kind: 'contractData',
      contractId: event.contractId,
      componentType: topics.componentType,
      entityId: data.entityId,
    },
  };
}

/** Decode every Sunvest event in an RPC page; non-Sunvest events are dropped. */
export function decodeSunvestEvents(events: SorobanRpcContractEvent[]): SunvestEventUpdate[] {
  const updates: SunvestEventUpdate[] = [];
  for (const event of events) {
    const update = decodeSunvestEvent(event);
    if (update !== null) updates.push(update);
  }
  return updates;
}

/** Decode a full `getEvents` page, preserving the page metadata. */
export function decodeSunvestPage(page: SorobanRpcEventsPage): {
  updates: SunvestEventUpdate[];
  page: SorobanRpcEventsPage;
} {
  return { updates: decodeSunvestEvents(page.events), page };
}

/** Narrow an update to the rich family (where a follow-up read is required). */
export function isRichComponentChanged(
  update: SunvestEventUpdate,
): update is RichComponentChangedUpdate {
  return update.family === 'rich';
}
