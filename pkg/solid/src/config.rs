use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SolidConfig {
    /// Minimum delay for each proposal
    pub min_proposal_duration: Duration,

    /// Maximum number of confirmed proposals to keep in history. This helps prevent us
    /// running out of memory if we receive a lot of proposals.
    pub max_proposal_history: u64,

    /// Amount of time to wait before we skip a leader
    pub skip_timeout: Duration,

    /// Amount of time to wait before we send another out of sync message
    pub out_of_sync_timeout: Duration,

    /// Threshold for accepting a proposal as confirmed
    pub accept_threshold: AcceptThreshold,

    /// Timeout to wait for missing proposal (i.e. we've received an accept for
    /// a proposal we don't have. In normal cases this could just be a race condition,
    /// and we will receive the proposal shortly after. However, if we don't receive
    /// the proposal after this timeout, we will send an out of sync event.
    pub missing_proposal_timeout: Duration,
}

impl Default for SolidConfig {
    fn default() -> Self {
        SolidConfig {
            min_proposal_duration: Duration::from_secs(1),
            max_proposal_history: 1024,
            skip_timeout: Duration::from_secs(5),
            out_of_sync_timeout: Duration::from_secs(60),
            accept_threshold: AcceptThreshold::MoreThanTwoThirds,
            missing_proposal_timeout: Duration::from_secs(5),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptThreshold {
    /// Accepts are required from more than two thirds of peers
    MoreThanTwoThirds,

    /// Accepts are required from a majority of peers
    Majority,
}

impl AcceptThreshold {
    /// Calculate the threshold based on the number of peers
    pub fn threshold(&self, peers: usize) -> usize {
        match self {
            AcceptThreshold::MoreThanTwoThirds => (peers * 2 / 3) + 1,
            AcceptThreshold::Majority => (peers / 2) + 1,
        }
    }

    /// Returns true only when threshold is exactly breached, and not at other times
    /// it is exceeded.
    pub fn is_exact_breach(&self, accepts: usize, peers: usize) -> bool {
        self.threshold(peers) == accepts
    }

    /// Inverse is the opposite of the threshold limit, so if the threshold is >2/3,
    /// then the inverse is >=1/3
    pub fn inverse_exceeded(&self, accepts: usize, peers: usize) -> bool {
        accepts > peers - self.threshold(peers)
    }

    pub fn is_exceeded(&self, accepts: usize, peers: usize) -> bool {
        accepts >= self.threshold(peers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_breach_two_thirds() {
        // 3 peers
        assert!(!AcceptThreshold::MoreThanTwoThirds.is_exact_breach(1, 3));
        assert!(!AcceptThreshold::MoreThanTwoThirds.is_exact_breach(2, 3));
        assert!(AcceptThreshold::MoreThanTwoThirds.is_exact_breach(3, 3));

        // 4 peers
        assert!(!AcceptThreshold::MoreThanTwoThirds.is_exact_breach(1, 4));
        assert!(!AcceptThreshold::MoreThanTwoThirds.is_exact_breach(2, 4));
        assert!(AcceptThreshold::MoreThanTwoThirds.is_exact_breach(3, 4));
        assert!(!AcceptThreshold::MoreThanTwoThirds.is_exact_breach(4, 4));

        // 5 peers
        assert!(!AcceptThreshold::MoreThanTwoThirds.is_exact_breach(1, 5));
        assert!(!AcceptThreshold::MoreThanTwoThirds.is_exact_breach(2, 5));
        assert!(!AcceptThreshold::MoreThanTwoThirds.is_exact_breach(3, 5));
        assert!(AcceptThreshold::MoreThanTwoThirds.is_exact_breach(4, 5));
        assert!(!AcceptThreshold::MoreThanTwoThirds.is_exact_breach(5, 5));
    }

    #[test]
    fn test_inverse_exceeded() {
        // 3 peers
        assert!(!AcceptThreshold::MoreThanTwoThirds.inverse_exceeded(0, 3));
        assert!(AcceptThreshold::MoreThanTwoThirds.inverse_exceeded(1, 3));
        assert!(AcceptThreshold::MoreThanTwoThirds.inverse_exceeded(2, 3));
        assert!(AcceptThreshold::MoreThanTwoThirds.inverse_exceeded(3, 3));

        // 4 peers
        assert!(!AcceptThreshold::MoreThanTwoThirds.inverse_exceeded(0, 4));
        assert!(!AcceptThreshold::MoreThanTwoThirds.inverse_exceeded(1, 4));
        assert!(AcceptThreshold::MoreThanTwoThirds.inverse_exceeded(2, 4));
        assert!(AcceptThreshold::MoreThanTwoThirds.inverse_exceeded(3, 4));
        assert!(AcceptThreshold::MoreThanTwoThirds.inverse_exceeded(4, 4));
    }

    #[test]
    fn test_is_exceeded() {
        assert!(!AcceptThreshold::MoreThanTwoThirds.is_exceeded(0, 1));

        // 3 peers
        assert!(!AcceptThreshold::MoreThanTwoThirds.is_exceeded(0, 3));
        assert!(!AcceptThreshold::MoreThanTwoThirds.is_exceeded(1, 3));
        assert!(!AcceptThreshold::MoreThanTwoThirds.is_exceeded(2, 3));
        assert!(AcceptThreshold::MoreThanTwoThirds.is_exceeded(3, 3));
    }
}
