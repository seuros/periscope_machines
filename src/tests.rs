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
        (issuer.scope(), issuer.issued(), issuer.remaining()),
        (9, 0, SEQUENCE_MAX)
    );
    assert_eq!(issuer.peek(), Ok(scoped(9, 1)));
    assert_eq!(issuer.peek(), Ok(scoped(9, 1))); // peeking issues nothing
    assert_eq!(issuer.issued(), 0);
    let mut last = 0;
    for expected in 1..=1_000 {
        let id = issuer.issue();
        assert_eq!(id, Ok(scoped(9, expected)));
        assert!(id.is_ok_and(|id| id > last));
        last = scoped(9, expected);
    }
    assert_eq!(
        (issuer.issued(), issuer.remaining()),
        (1_000, SEQUENCE_MAX - 1_000)
    );
    assert_eq!(Issuer::default(), Issuer::new(0));
}

#[test]
fn zero_is_never_issued_and_never_owned() {
    let mut issuer = Issuer::new(0);
    assert!(!issuer.owns(0));
    assert_eq!(issuer.issue(), Ok(1));
    assert!(!issuer.owns(0));
    assert!(issuer.owns(1));
}

#[test]
fn ownership_is_scope_and_issued_range() {
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
    // Two issuers left in the same scope are indistinguishable: the caller's job.
    let mut twin = Issuer::new(1);
    assert_eq!(twin.issue(), a);
}

#[test]
fn the_last_id_is_issued_then_the_scope_is_spent_and_unchanged() {
    let mut issuer = Issuer::new(u16::MAX);
    issuer.issued = SEQUENCE_MAX - 1;
    assert_eq!(issuer.remaining(), 1);
    assert_eq!(issuer.issue(), Ok(u64::MAX));
    let before = issuer;
    assert_eq!(issuer.peek(), Err(Refusal::Exhausted));
    assert_eq!(issuer.issue(), Err(Refusal::Exhausted));
    assert_eq!(issuer, before);
    assert_eq!(issuer.remaining(), 0);
    assert!(issuer.owns(u64::MAX));
}

#[test]
fn everything_is_usable_in_constants() {
    const ID: u64 = scoped(4, 2);
    const SCOPE: u16 = scope_of(ID);
    const FIRST: Result<u64, Refusal> = Issuer::new(4).peek();
    assert_eq!((ID, SCOPE, FIRST), (scoped(4, 2), 4, Ok(scoped(4, 1))));
}
