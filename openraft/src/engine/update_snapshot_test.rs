use maplit::btreeset;
use pretty_assertions::assert_eq;

use crate::engine::Engine;
use crate::EffectiveMembership;
use crate::LeaderId;
use crate::LogId;
use crate::Membership;
use crate::MetricsChangeFlags;
use crate::SnapshotMeta;

fn log_id(term: u64, index: u64) -> LogId<u64> {
    LogId::<u64> {
        leader_id: LeaderId { term, node_id: 1 },
        index,
    }
}

fn m12() -> Membership<u64, ()> {
    Membership::<u64, ()>::new(vec![btreeset! {1,2}], None)
}

fn m1234() -> Membership<u64, ()> {
    Membership::<u64, ()>::new(vec![btreeset! {1,2,3,4}], None)
}

fn eng() -> Engine<u64, ()> {
    Engine::<u64, ()> {
        snapshot_meta: SnapshotMeta {
            last_log_id: Some(log_id(2, 2)),
            last_membership: EffectiveMembership::new(Some(log_id(1, 1)), m12()),
            snapshot_id: "1-2-3-4".to_string(),
        },
        ..Default::default()
    }
}

#[test]
fn test_update_snapshot_no_update() -> anyhow::Result<()> {
    // snapshot will not be updated because of equal or less `last_log_id`.
    let mut eng = eng();

    let got = eng.update_snapshot(SnapshotMeta {
        last_log_id: Some(log_id(2, 2)),
        last_membership: EffectiveMembership::new(Some(log_id(1, 1)), m1234()),
        snapshot_id: "1-2-3-4".to_string(),
    });

    assert_eq!(false, got);

    assert_eq!(
        SnapshotMeta {
            last_log_id: Some(log_id(2, 2)),
            last_membership: EffectiveMembership::new(Some(log_id(1, 1)), m12()),
            snapshot_id: "1-2-3-4".to_string(),
        },
        eng.snapshot_meta
    );

    assert_eq!(
        MetricsChangeFlags {
            replication: false,
            local_data: false,
            cluster: false,
        },
        eng.metrics_flags
    );

    assert_eq!(0, eng.commands.len());

    Ok(())
}

#[test]
fn test_update_snapshot_updated() -> anyhow::Result<()> {
    // snapshot will be updated to a new one with greater `last_log_id`.
    let mut eng = eng();

    let got = eng.update_snapshot(SnapshotMeta {
        last_log_id: Some(log_id(2, 3)),
        last_membership: EffectiveMembership::new(Some(log_id(2, 2)), m1234()),
        snapshot_id: "1-2-3-4".to_string(),
    });

    assert_eq!(true, got);

    assert_eq!(
        SnapshotMeta {
            last_log_id: Some(log_id(2, 3)),
            last_membership: EffectiveMembership::new(Some(log_id(2, 2)), m1234()),
            snapshot_id: "1-2-3-4".to_string(),
        },
        eng.snapshot_meta
    );

    assert_eq!(
        MetricsChangeFlags {
            replication: false,
            local_data: true,
            cluster: false,
        },
        eng.metrics_flags
    );

    assert_eq!(0, eng.commands.len());

    Ok(())
}
