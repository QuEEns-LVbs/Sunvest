import {
  Sunvest_NAMESPACE,
  decodeSunvestTopics,
  type SunvestEventFamily,
  type SorobanRpcContractEvent,
} from './events.ts';
import { encodeSymbolBase64 } from './xdr.ts';

/**
 * A `TopicFilter` segment list: one base64 `ScVal` per topic position.
 * `"*"` is the Soroban RPC wildcard for a single position.
 */
export type SorobanTopicFilter = string[];

export interface SorobanEventFilter {
  type: 'contract';
  contractIds?: string[];
  topics?: SorobanTopicFilter[];
}

export interface SunvestTopicFilterOptions {
  /** Defaults to all three families. */
  families?: readonly SunvestEventFamily[];
  /** Restrict to one component type, e.g. `"position"`. */
  componentType?: string;
}

const FAMILIES: readonly SunvestEventFamily[] = ['set', 'del', 'rich'];
const NAMESPACE_TOPIC = encodeSymbolBase64(Sunvest_NAMESPACE);

const FAMILY_TOPICS: Record<SunvestEventFamily, string> = {
  set: encodeSymbolBase64('set'),
  del: encodeSymbolBase64('del'),
  rich: encodeSymbolBase64('rich'),
};

/**
 * Build the `topics` array for a Soroban RPC `getEvents` filter.
 *
 * Topic positions are ANDed inside a filter and the filters themselves are
 * ORed, so `[[Sunvest,set],[Sunvest,del],[Sunvest,rich]]` asks for exactly the three
 * Sunvest families. Adding a `componentType` appends it as a third, ANDed
 * position.
 */
export function buildSunvestTopicFilters(
  options: SunvestTopicFilterOptions = {},
): SorobanTopicFilter[] {
  const families = options.families ?? FAMILIES;
  const componentTopic =
    options.componentType === undefined ? null : encodeSymbolBase64(options.componentType);
  return families.map((family) => {
    const filter = [NAMESPACE_TOPIC, FAMILY_TOPICS[family]];
    if (componentTopic !== null) filter.push(componentTopic);
    return filter;
  });
}

/** Convenience wrapper that returns a complete contract event filter. */
export function SunvestEventFilter(options: SunvestTopicFilterOptions & { contractIds?: string[] } = {}): SorobanEventFilter {
  const { contractIds, ...topicOptions } = options;
  const filter: SorobanEventFilter = {
    type: 'contract',
    topics: buildSunvestTopicFilters(topicOptions),
  };
  if (contractIds !== undefined && contractIds.length > 0) filter.contractIds = contractIds;
  return filter;
}

/**
 * Client-side mirror of {@link buildSunvestTopicFilters}, for events that were
 * fetched with a broader filter (or a wildcard `*` component type).
 */
export function eventMatchesSunvestTopics(
  event: SorobanRpcContractEvent,
  options: SunvestTopicFilterOptions = {},
): boolean {
  const topics = decodeSunvestTopics(event);
  if (topics === null) return false;
  const families = options.families ?? FAMILIES;
  if (!families.includes(topics.family)) return false;
  if (options.componentType !== undefined && topics.componentType !== options.componentType) {
    return false;
  }
  return true;
}

/** The raw base64 topics for the `Sunvest` namespace, handy in docs/tests. */
export const Sunvest_TOPICS = {
  namespace: NAMESPACE_TOPIC,
  ...FAMILY_TOPICS,
} as const;
