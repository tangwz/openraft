use crate::engine::LogIdList;
use crate::internal_server_state::InternalServerState;
use crate::leader::Leader;
use crate::node::Node;
use crate::raft_types::RaftLogId;
use crate::LogId;
use crate::LogIdOptionExt;
use crate::MembershipState;
use crate::NodeId;
use crate::ServerState;
use crate::Vote;

/// A struct used to represent the raft state which a Raft node needs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RaftState<NID, N>
where
    NID: NodeId,
    N: Node,
{
    /// The vote state of this node.
    pub vote: Vote<NID>,

    /// The LogId of the last log committed(AKA applied) to the state machine.
    ///
    /// - Committed means: a log that is replicated to a quorum of the cluster and it is of the term of the leader.
    ///
    /// - A quorum could be a uniform quorum or joint quorum.
    pub committed: Option<LogId<NID>>,

    /// All log ids this node has.
    pub log_ids: LogIdList<NID>,

    /// The latest cluster membership configuration found, in log or in state machine.
    pub membership_state: MembershipState<NID, N>,

    // --
    // -- volatile fields: they are not persisted.
    // --
    /// The internal server state used by Engine.
    pub(crate) internal_server_state: InternalServerState<NID>,

    pub server_state: ServerState,
}

impl<NID, N> RaftState<NID, N>
where
    NID: NodeId,
    N: Node,
{
    /// Append a list of `log_id`.
    ///
    /// The log ids in the input has to be continuous.
    pub(crate) fn extend_log_ids_from_same_leader<'a, LID: RaftLogId<NID> + 'a>(&mut self, new_log_ids: &[LID]) {
        self.log_ids.extend_from_same_leader(new_log_ids)
    }

    #[allow(dead_code)]
    pub(crate) fn extend_log_ids<'a, LID: RaftLogId<NID> + 'a>(&mut self, new_log_id: &[LID]) {
        self.log_ids.extend(new_log_id)
    }

    /// Get the log id at the specified index.
    ///
    /// It will return `last_purged_log_id` if index is at the last purged index.
    pub(crate) fn get_log_id(&self, index: u64) -> Option<LogId<NID>> {
        self.log_ids.get(index)
    }

    /// Return if a log id exists.
    ///
    /// It assumes a committed log will always be chosen, according to raft spec.
    #[allow(dead_code)]
    pub(crate) fn has_log_id(&self, log_id: &LogId<NID>) -> bool {
        if log_id.index < self.committed.next_index() {
            debug_assert!(Some(*log_id) <= self.committed);
            return true;
        }

        // The local log id exists at the index and is same as the input.
        if let Some(local) = self.get_log_id(log_id.index) {
            *log_id == local
        } else {
            false
        }
    }

    /// The last known log id in the store.
    pub(crate) fn last_log_id(&self) -> Option<LogId<NID>> {
        self.log_ids.last().cloned()
    }

    /// The greatest log id that has been purged after being applied to state machine, i.e., the oldest known log id.
    ///
    /// The range of log entries that exist in storage is `(last_purged_log_id, last_log_id]`,
    /// left open and right close.
    ///
    /// `last_purged_log_id == last_log_id` means there is no log entry in the storage.
    pub(crate) fn last_purged_log_id(&self) -> Option<LogId<NID>> {
        self.log_ids.first().cloned()
    }

    /// Create a new Leader, when raft enters candidate state.
    /// In openraft, Leader and Candidate shares the same state.
    pub(crate) fn new_leader(&mut self) {
        let em = &self.membership_state.effective;
        self.internal_server_state =
            InternalServerState::Leading(Leader::new(em.membership.to_quorum_set(), em.learner_ids()));
    }

    /// Return true if the currently effective membership is committed.
    pub(crate) fn is_membership_committed(&self) -> bool {
        self.committed >= self.membership_state.effective.log_id
    }

    /// Update field `committed` if the input is greater.
    /// If updated, it returns the previous value in a `Some()`.
    pub(crate) fn update_committed(&mut self, committed: &Option<LogId<NID>>) -> Option<Option<LogId<NID>>> {
        if committed > &self.committed {
            let prev = self.committed;

            self.committed = *committed;

            // TODO(xp): use a vec to store committed and effective membership.
            if self.committed >= self.membership_state.effective.log_id {
                self.membership_state.committed = self.membership_state.effective.clone();
            }

            Some(prev)
        } else {
            None
        }
    }
}
