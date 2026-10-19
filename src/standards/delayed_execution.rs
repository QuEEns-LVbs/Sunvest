use soroban_sdk::{contracttype, Bytes, Env, Symbol, Vec};

use super::error::StandardsError;

const OPERATION_PREFIX: &str = "std_delay";
const OPERATION_IDS_PREFIX: &str = "std_delay_ids";
const NONCE_PREFIX: &str = "std_delay_n";

/// Storage-backed delayed execution queue.
#[derive(Clone, Debug)]
pub struct DelayedExecutionPolicy {
    id: Symbol,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DelayedOperation {
    pub operation_id: u64,
    pub action: Symbol,
    pub payload: Bytes,
    pub scheduled_at: u64,
    pub not_before: u64,
    pub expires_at: u64,
    pub executed: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DelayedExecutionScheduledEvent {
    pub operation_id: u64,
    pub action: Symbol,
    pub not_before: u64,
    pub expires_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DelayedExecutionCancelledEvent {
    pub operation_id: u64,
    pub action: Symbol,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DelayedExecutionExecutedEvent {
    pub operation_id: u64,
    pub action: Symbol,
    pub executed_at: u64,
}

impl DelayedExecutionPolicy {
    pub fn new(id: Symbol) -> Self {
        Self { id }
    }

    pub fn schedule(
        &self,
        env: &Env,
        action: Symbol,
        payload: Bytes,
        delay: u64,
        ttl: u64,
    ) -> Result<DelayedExecutionScheduledEvent, StandardsError> {
        let now = env.ledger().timestamp();
        let not_before = now
            .checked_add(delay)
            .ok_or(StandardsError::ScheduleOverflow)?;
        let expires_at = not_before
            .checked_add(ttl)
            .ok_or(StandardsError::ScheduleOverflow)?;
        let operation_id = self.next_operation_id(env)?;
        let operation = DelayedOperation {
            operation_id,
            action: action.clone(),
            payload,
            scheduled_at: now,
            not_before,
            expires_at,
            executed: false,
        };

        env.storage()
            .persistent()
            .set(&self.operation_key(env, operation_id), &operation);

        let mut ids = self.operation_ids(env);
        ids.push_back(operation_id);
        env.storage()
            .persistent()
            .set(&self.operation_ids_key(env), &ids);

        Ok(DelayedExecutionScheduledEvent {
            operation_id,
            action,
            not_before: operation.not_before,
            expires_at: operation.expires_at,
        })
    }

    pub fn operation(&self, env: &Env, operation_id: u64) -> Option<DelayedOperation> {
        env.storage()
            .persistent()
            .get(&self.operation_key(env, operation_id))
    }

    pub fn pending_operations(&self, env: &Env) -> Vec<DelayedOperation> {
        let ids = self.operation_ids(env);
        let mut operations = Vec::new(env);

        for i in 0..ids.len() {
            if let Some(operation_id) = ids.get(i) {
                if let Some(operation) = self.operation(env, operation_id) {
                    if !operation.executed {
                        operations.push_back(operation);
                    }
                }
            }
        }

        operations
    }

    pub fn cancel(
        &self,
        env: &Env,
        operation_id: u64,
    ) -> Result<DelayedExecutionCancelledEvent, StandardsError> {
        let operation = self
            .operation(env, operation_id)
            .ok_or(StandardsError::OperationNotFound)?;

        if operation.executed {
            return Err(StandardsError::OperationAlreadyExecuted);
        }

        self.remove_operation(env, operation_id);

        Ok(DelayedExecutionCancelledEvent {
            operation_id,
            action: operation.action,
        })
    }

    pub fn execute_ready(
        &self,
        env: &Env,
        operation_id: u64,
    ) -> Result<DelayedExecutionExecutedEvent, StandardsError> {
        let mut operation = self
            .operation(env, operation_id)
            .ok_or(StandardsError::OperationNotFound)?;

        if operation.executed {
            return Err(StandardsError::OperationAlreadyExecuted);
        }

        let now = env.ledger().timestamp();
        if now < operation.not_before {
            return Err(StandardsError::OperationNotReady);
        }
        if now > operation.expires_at {
            return Err(StandardsError::OperationExpired);
        }

        operation.executed = true;
        env.storage()
            .persistent()
            .set(&self.operation_key(env, operation_id), &operation);
        self.remove_operation_id(env, operation_id);

        Ok(DelayedExecutionExecutedEvent {
            operation_id,
            action: operation.action,
            executed_at: now,
        })
    }

    fn next_operation_id(&self, env: &Env) -> Result<u64, StandardsError> {
        let key = self.nonce_key(env);
        let next = env
            .storage()
            .persistent()
            .get::<_, u64>(&key)
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(StandardsError::ScheduleOverflow)?;
        env.storage().persistent().set(&key, &next);
        Ok(next)
    }

    fn remove_operation(&self, env: &Env, operation_id: u64) {
        env.storage()
            .persistent()
            .remove(&self.operation_key(env, operation_id));
        self.remove_operation_id(env, operation_id);
    }

    fn remove_operation_id(&self, env: &Env, operation_id: u64) {
        let ids = self.operation_ids(env);
        let mut retained = Vec::new(env);

        for i in 0..ids.len() {
            if let Some(candidate) = ids.get(i) {
                if candidate != operation_id {
                    retained.push_back(candidate);
                }
            }
        }

        env.storage()
            .persistent()
            .set(&self.operation_ids_key(env), &retained);
    }

    fn operation_ids(&self, env: &Env) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&self.operation_ids_key(env))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn operation_key(&self, env: &Env, operation_id: u64) -> (Symbol, Symbol, u64) {
        (
            Symbol::new(env, OPERATION_PREFIX),
            self.id.clone(),
            operation_id,
        )
    }

    fn operation_ids_key(&self, env: &Env) -> (Symbol, Symbol) {
        (Symbol::new(env, OPERATION_IDS_PREFIX), self.id.clone())
    }

    fn nonce_key(&self, env: &Env) -> (Symbol, Symbol) {
        (Symbol::new(env, NONCE_PREFIX), self.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{contract, contractimpl, symbol_short, testutils::Ledger as _};

    #[contract]
    struct TestContract;

    #[contractimpl]
    impl TestContract {}

    fn setup() -> (Env, soroban_sdk::Address, DelayedExecutionPolicy) {
        let env = Env::default();
        let contract_id = env.register(TestContract, ());
        env.ledger().with_mut(|li| li.timestamp = 100);
        (
            env,
            contract_id,
            DelayedExecutionPolicy::new(symbol_short!("delay")),
        )
    }

    fn payload(env: &Env) -> Bytes {
        Bytes::from_array(env, &[1])
    }

    #[test]
    fn schedule_rejects_delay_overflow() {
        let (env, contract_id, module) = setup();
        env.as_contract(&contract_id, || {
            let result = module.schedule(&env, symbol_short!("OP"), payload(&env), u64::MAX, 0);
            assert_eq!(result, Err(StandardsError::ScheduleOverflow));
            assert_eq!(module.pending_operations(&env).len(), 0);
        });
    }

    #[test]
    fn schedule_rejects_ttl_overflow() {
        let (env, contract_id, module) = setup();
        env.as_contract(&contract_id, || {
            let result = module.schedule(&env, symbol_short!("OP"), payload(&env), 10, u64::MAX);
            assert_eq!(result, Err(StandardsError::ScheduleOverflow));
            assert_eq!(module.pending_operations(&env).len(), 0);
        });
    }

    #[test]
    fn failed_schedule_does_not_consume_an_operation_id() {
        let (env, contract_id, module) = setup();
        env.as_contract(&contract_id, || {
            assert!(module
                .schedule(&env, symbol_short!("OP"), payload(&env), u64::MAX, 0)
                .is_err());
            let scheduled = module
                .schedule(&env, symbol_short!("OP"), payload(&env), 1, 1)
                .unwrap();
            assert_eq!(scheduled.operation_id, 1);
        });
    }

    #[test]
    fn schedule_rejects_exhausted_id_counter() {
        let (env, contract_id, module) = setup();
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&module.nonce_key(&env), &u64::MAX);
            let result = module.schedule(&env, symbol_short!("OP"), payload(&env), 1, 1);
            assert_eq!(result, Err(StandardsError::ScheduleOverflow));
            assert_eq!(module.pending_operations(&env).len(), 0);
        });
    }

    #[test]
    fn cancel_rejects_executed_operation_and_keeps_storage() {
        let (env, contract_id, module) = setup();
        env.as_contract(&contract_id, || {
            let scheduled = module
                .schedule(&env, symbol_short!("OP"), payload(&env), 5, 5)
                .unwrap();
            env.ledger().with_mut(|li| li.timestamp = 105);
            module.execute_ready(&env, scheduled.operation_id).unwrap();
            let before = module.operation(&env, scheduled.operation_id);

            let result = module.cancel(&env, scheduled.operation_id);

            assert_eq!(result, Err(StandardsError::OperationAlreadyExecuted));
            assert_eq!(module.operation(&env, scheduled.operation_id), before);
            assert!(before.unwrap().executed);
        });
    }

    #[test]
    fn cancel_removes_pending_operation() {
        let (env, contract_id, module) = setup();
        env.as_contract(&contract_id, || {
            let scheduled = module
                .schedule(&env, symbol_short!("OP"), payload(&env), 5, 5)
                .unwrap();
            let event = module.cancel(&env, scheduled.operation_id).unwrap();
            assert_eq!(event.operation_id, scheduled.operation_id);
            assert_eq!(module.operation(&env, scheduled.operation_id), None);
        });
    }
}
