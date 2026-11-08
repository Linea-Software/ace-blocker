//! Pure WFP filter installation plan shared by Windows enforcement and host-independent tests.
//!
//! IPv6 interception depends on usable physical egress at startup, but IPv6
//! *enforcement* must never depend on that snapshot: a network change can add
//! IPv6 connectivity without recreating the adapter. Without the IPv6 block
//! a protected browser would bypass ACE on the newly available route.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilterStep {
    PermitTunV4,
    PermitLocalV4,
    BlockOtherV4,
    PermitTunV6,
    PermitLocalV6,
    BlockOtherV6,
}

/// Returns the complete per-application WFP ALE_AUTH_CONNECT policy.
///
/// Every protected browser gets both families' local permits and final blocks.
/// The WinTun permit for IPv6 exists only when that family is actually routed
/// through and serviced by the tunnel. When IPv6 capture is unavailable, this
/// policy intentionally denies *external* IPv6 for protected applications,
/// including if IPv6 connectivity appears after ACE started.
pub(crate) fn filter_steps(ipv6_capture_enabled: bool) -> impl Iterator<Item = FilterStep> {
    [
        Some(FilterStep::PermitTunV4),
        Some(FilterStep::PermitLocalV4),
        Some(FilterStep::BlockOtherV4),
        ipv6_capture_enabled.then_some(FilterStep::PermitTunV6),
        Some(FilterStep::PermitLocalV6),
        Some(FilterStep::BlockOtherV6),
    ]
    .into_iter()
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_families_capture_when_ipv6_egress_is_available() {
        assert_eq!(
            filter_steps(true).collect::<Vec<_>>(),
            [
                FilterStep::PermitTunV4,
                FilterStep::PermitLocalV4,
                FilterStep::BlockOtherV4,
                FilterStep::PermitTunV6,
                FilterStep::PermitLocalV6,
                FilterStep::BlockOtherV6,
            ]
        );
    }

    #[test]
    fn absent_ipv6_capture_still_installs_ipv6_local_permit_and_external_block() {
        assert_eq!(
            filter_steps(false).collect::<Vec<_>>(),
            [
                FilterStep::PermitTunV4,
                FilterStep::PermitLocalV4,
                FilterStep::BlockOtherV4,
                FilterStep::PermitLocalV6,
                FilterStep::BlockOtherV6,
            ]
        );
    }

    #[test]
    fn each_family_always_ends_in_a_catch_all_block() {
        for ipv6_capture in [false, true] {
            let policy = filter_steps(ipv6_capture).collect::<Vec<_>>();
            assert!(policy.contains(&FilterStep::BlockOtherV4));
            assert_eq!(policy.last(), Some(&FilterStep::BlockOtherV6));
            assert_eq!(policy.contains(&FilterStep::PermitTunV6), ipv6_capture);
        }
    }
}
