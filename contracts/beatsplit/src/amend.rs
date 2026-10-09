//! Amendment workflow for BeatSplit splits.
//!
//! An amendment allows recipients to change the recipient list of an active split.
//! All current recipients must approve unanimously for the amendment to take effect.
//!
//! Rules:
//! - Any recipient can propose an amendment.
//! - Only one proposal can be open at a time.
//! - All current recipients must approve.
//! - If the proposal changes, approvals reset.
//! - Applying an amendment increments the split's version.
//! - Locked splits cannot be amended.

use soroban_sdk::{contracttype, Address, Vec};

use crate::types::Recipient;

/// A pending amendment proposal for a split.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct AmendmentProposal {
    /// The split ID this proposal applies to.
    pub split_id: u64,
    /// Address of the recipient who proposed the amendment.
    pub proposer: Address,
    /// The proposed new recipient list.
    pub new_recipients: Vec<Recipient>,
    /// Addresses of recipients who have approved this proposal.
    pub approvals: Vec<Address>,
    /// Version of the split when this proposal was created.
    /// If the split's version changes, this proposal becomes stale.
    pub base_version: u32,
}

impl AmendmentProposal {
    /// Check if all current recipients have approved this proposal.
    pub fn is_fully_approved(&self, current_recipients: &Vec<Recipient>) -> bool {
        if current_recipients.len() != self.approvals.len() {
            return false;
        }
        for recipient in current_recipients.iter() {
            if !self.approvals.iter().any(|a| a == recipient.addr) {
                return false;
            }
        }
        true
    }

    /// Check if a recipient has already approved this proposal.
    pub fn has_approved(&self, addr: &Address) -> bool {
        self.approvals.iter().any(|a| a == *addr)
    }

    /// Add an approval from a recipient.
    pub fn add_approval(&mut self, addr: Address) {
        if !self.has_approved(&addr) {
            self.approvals.push_back(addr);
        }
    }
}
