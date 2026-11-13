# periscope-machines

> **AMS Grubb**, NCC-4236, *Grubb*-class observation tender.

Deck-watch flight manual: stamp every id with the ship that issued it, so no
ship ever mistakes another's handle for its own.

Howard Grubb built the periscopes that let a submarine tell what was in front
of it without surfacing. The tender does the same for an id: one look at the
top sixteen bits says whose it is. The fleet needed it because ids are plain
integers counted from one. Two wheels issue the same handles, two bandits the
same tickets, and a handle given to the wrong one names something real there.
An independent review found exactly that, twice.

Allocation-free `no_std`, no dependencies, no `unsafe`, every function a
`const fn`. This is a Rust-only support crate with no Ruby gem.

```rust
use periscope_machines::{Issuer, Refusal, scope_of, scoped, sequence_of};

// Two issuers, two scopes, the same calls: never the same id.
let mut near = Issuer::new(1);
let mut far = Issuer::new(2);
let (a, b) = (near.issue(), far.issue());
assert_eq!(a, Ok(scoped(1, 1)));
assert_ne!(a, b);

// An issuer knows what it handed out, and nothing else.
assert!(near.owns(scoped(1, 1)));
assert!(!near.owns(scoped(2, 1))); // another scope
assert!(!near.owns(scoped(1, 2))); // not issued yet

// Peek, check your own capacity, then issue.
let next = near.peek();
assert_eq!(next, Ok(scoped(1, 2)));
assert_eq!(near.issue(), next);

assert_eq!((scope_of(scoped(7, 42)), sequence_of(scoped(7, 42))), (7, 42));
let _: Refusal = Refusal::Exhausted;
```

## Contract

- **Layout.** An id is a `u64`: the scope in the top `SCOPE_BITS = 16`, a
  sequence number in the low `SEQUENCE_BITS = 48`. `scoped`, `scope_of` and
  `sequence_of` are the only functions that know this.
- **Sequence.** An `Issuer` counts from one and never reissues. After
  `SEQUENCE_MAX` (2^48 - 1) ids it answers `Refusal::Exhausted` and is left
  unchanged. Raw zero is never issued, so it can stand for "no id".
- **Scope zero is the plain counter.** `Issuer::new(0)` issues 1, 2, 3, so a
  core that had unscoped ids keeps the same values for its default scope.
- **`peek` issues nothing.** It returns what the next `issue` will return, for
  a core that must check its own tables before committing to an id.
- **`owns` proves origin, not liveness.** It is true for an id of this scope
  whose sequence number was issued. Whether the id is still live is the
  caller's table. Proving origin is what lets a core tell "this was mine and
  is gone" from "this was never mine".
- **Scopes are the caller's.** An issuer reads no clock and no random source,
  so it cannot pick a scope. Two issuers given the same scope issue the same
  ids. Give every instance whose ids could meet its own scope.

## Not Included

No registry of scopes, no random ids, no time-ordered ids, no wider scopes.
Sixteen bits is a deployment-sized number of instances, not a global namespace.

## License

MIT
