use super::*;

#[test]
fn layout_round_trips_and_keeps_scope_and_sequence_apart() {
    assert_eq!((SCOPE_BITS, SEQUENCE_BITS), (16, 48));
    assert_eq!(SEQUENCE_MAX, 0x0000_FFFF_FFFF_FFFF);
    for (scope, sequence) in [(0, 0), (0, 1), (1, 1), (7, 42), (u16::MAX, SEQUENCE_MAX)] {
        let raw = scoped(scope, sequence);
        assert_eq!((scope_of(raw), sequence_of(raw)), (scope, sequence));
    }
    assert_eq!(scoped(0, 5), 5); // scope zero is the plain counter
    assert_eq!(scoped(1, 1), (1 << 48) | 1);
    assert_eq!(scoped(u16::MAX, SEQUENCE_MAX), u64::MAX);
    // Sequence bits above the field cannot leak into the scope.
    assert_eq!(scoped(3, u64::MAX), scoped(3, SEQUENCE_MAX));
    assert_eq!(scope_of(scoped(3, u64::MAX)), 3);
}

#[test]
fn an_issuer_counts_from_one_and_never_reissues() {
    let mut issuer = Issuer::new(9);
    assert_eq!(
        (
            issuer.scope(),
            issuer.issued(),
            issuer.remaining(),
            issuer.last()
        ),
        (9, 0, SEQUENCE_MAX, None)
    );
    assert_eq!(issuer.peek(), Ok(scoped(9, 1)));
    assert_eq!(issuer.peek(), Ok(scoped(9, 1))); // peeking issues nothing
    assert_eq!(issuer.issued(), 0);
    let mut last = 0;
    for expected in 1..=1_000 {
        let id = issuer.issue();
        assert_eq!(id, Ok(scoped(9, expected)));
        assert!(id.is_ok_and(|id| id > last));
        assert_eq!(issuer.last().map(Ok), Some(id));
        last = scoped(9, expected);
    }
    assert_eq!(
        (issuer.issued(), issuer.remaining()),
        (1_000, SEQUENCE_MAX - 1_000)
    );
    assert_eq!(Issuer::default(), Issuer::new(0));
}

#[test]
fn ownership_is_scope_and_issued_range_and_never_zero() {
    let mut near = Issuer::new(1);
    let mut far = Issuer::new(2);
    let (a, b) = (near.issue(), far.issue());
    assert_eq!(sequence_of(scoped(1, 1)), sequence_of(scoped(2, 1)));
    assert_ne!(a, b); // same calls, different scopes, different ids
    assert!(near.owns(scoped(1, 1)) && far.owns(scoped(2, 1)));
    assert!(!near.owns(scoped(2, 1)) && !far.owns(scoped(1, 1)));
    assert!(!near.owns(scoped(1, 2))); // not issued yet
    assert_eq!(near.issue(), Ok(scoped(1, 2)));
    assert!(near.owns(scoped(1, 2)));
    // Sequence zero is never issued, so never owned.
    assert!(!near.owns(scoped(1, 0)));
    let mut plain = Issuer::new(0);
    assert_eq!(plain.issue(), Ok(1));
    assert!(!plain.owns(0) && plain.owns(1));
    // Two issuers left in the same scope are indistinguishable: the caller's job.
    let mut twin = Issuer::new(1);
    assert_eq!(twin.issue(), a);
}

#[test]
fn a_resumed_issuer_continues_and_a_spent_scope_refuses_unchanged() {
    let mut first = Issuer::new(5);
    for _ in 0..3 {
        assert!(first.issue().is_ok());
    }
    let mut resumed = Issuer::resumed(first.scope(), first.issued());
    assert_eq!(resumed, Some(first));
    assert_eq!(
        resumed.map(|issuer| issuer.last()),
        Some(Some(scoped(5, 3)))
    );
    assert_eq!(resumed.as_mut().map(Issuer::issue), Some(Ok(scoped(5, 4))));
    assert_eq!(resumed.map(|issuer| issuer.owns(scoped(5, 3))), Some(true));
    assert_eq!(Issuer::resumed(5, 0), Some(Issuer::new(5)));
    // A fully spent scope can be resumed and stays spent; beyond it is not a state.
    let spent = Issuer::resumed(5, SEQUENCE_MAX);
    assert_eq!(
        spent.map(|issuer| (issuer.peek(), issuer.remaining(), issuer.last())),
        Some((Err(Refusal::Exhausted), 0, Some(scoped(5, SEQUENCE_MAX))))
    );
    let mut spent_issuer = spent;
    assert_eq!(
        spent_issuer.as_mut().map(Issuer::issue),
        Some(Err(Refusal::Exhausted))
    );
    assert_eq!(spent_issuer, spent);
    assert_eq!(Issuer::resumed(5, SEQUENCE_MAX + 1), None);

    // The last id is issued, then the scope is spent; a refusal changes nothing.
    let Some(mut issuer) = Issuer::resumed(u16::MAX, SEQUENCE_MAX - 1) else {
        panic!("SEQUENCE_MAX - 1 is a count an issuer reaches");
    };
    assert_eq!(issuer.remaining(), 1);
    assert_eq!(issuer.issue(), Ok(u64::MAX));
    let before = issuer;
    assert_eq!(issuer.peek(), Err(Refusal::Exhausted));
    assert_eq!(issuer.issue(), Err(Refusal::Exhausted));
    assert_eq!(issuer, before);
    assert_eq!((issuer.remaining(), issuer.last()), (0, Some(u64::MAX)));
    assert!(issuer.owns(u64::MAX));
}

#[test]
fn everything_is_usable_in_constants() {
    const ID: u64 = scoped(4, 2);
    const SCOPE: u16 = scope_of(ID);
    const FIRST: Result<u64, Refusal> = Issuer::new(4).peek();
    const NONE: Option<u64> = Issuer::new(4).last();
    const RESUMED: Option<Issuer> = Issuer::resumed(1, 2);
    const RESTORED: Option<Issuer> = Issuer::restored(1, Some(scoped(1, 2)));
    assert_eq!(RESTORED, RESUMED);
    assert_eq!(
        (ID, SCOPE, FIRST, NONE),
        (scoped(4, 2), 4, Ok(scoped(4, 1)), None)
    );
    assert_eq!(RESUMED.map(|issuer| issuer.issued()), Some(2));
}

#[test]
fn a_restored_issuer_continues_after_its_last_id() {
    // Nothing issued is spelled `None`, and restores a fresh issuer.
    assert_eq!(Issuer::restored(5, None), Some(Issuer::new(5)));
    assert_eq!(
        Issuer::restored(5, Issuer::new(5).last()),
        Some(Issuer::new(5))
    );

    let mut first = Issuer::new(5);
    for _ in 0..3 {
        assert!(first.issue().is_ok());
    }
    let mut restored = Issuer::restored(first.scope(), first.last());
    assert_eq!(restored, Some(first));
    assert_eq!(restored, Issuer::resumed(5, 3));
    assert_eq!(restored.as_mut().map(Issuer::issue), Some(Ok(scoped(5, 4))));
    assert_eq!(restored.map(|issuer| issuer.owns(scoped(5, 3))), Some(true));

    // `last` and `restored` round-trip at both ends of the sequence.
    for issued in [0, 1, 2, SEQUENCE_MAX - 1, SEQUENCE_MAX] {
        for scope in [0, 1, u16::MAX] {
            let issuer = Issuer::resumed(scope, issued);
            assert_eq!(
                issuer.and_then(|issuer| Issuer::restored(scope, issuer.last())),
                issuer
            );
        }
    }
    let spent = Issuer::restored(u16::MAX, Some(u64::MAX));
    assert_eq!(
        spent.map(|issuer| (issuer.peek(), issuer.remaining())),
        Some((Err(Refusal::Exhausted), 0))
    );
}

#[test]
fn restored_refuses_an_id_that_is_not_of_the_scope() {
    // Another scope: the scope is the caller's, never read out of the id.
    assert_eq!(Issuer::restored(5, Some(scoped(6, 3))), None);
    // An id that lost its top bits cannot land a scoped issuer in scope zero.
    assert_eq!(Issuer::restored(5, Some(sequence_of(scoped(5, 3)))), None);
    assert_eq!(Issuer::restored(0, Some(scoped(5, 3))), None);
    // A zero sequence is no id, in any scope: not even the `0` of scope zero.
    assert_eq!(Issuer::restored(0, Some(0)), None);
    assert_eq!(Issuer::restored(5, Some(scoped(5, 0))), None);
    // Every other id of the scope restores: no sequence is out of reach.
    assert_eq!(
        Issuer::restored(0, Some(1)).map(|issuer| issuer.last()),
        Some(Some(1))
    );
}
