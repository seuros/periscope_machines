#![no_std]
#![doc = include_str!("../README.md")]

use core::fmt;

/// Bits of an id that carry the issuer's scope: the top sixteen.
pub const SCOPE_BITS: u32 = 16;

/// Bits of an id that count what the issuer has issued: the low forty-eight.
pub const SEQUENCE_BITS: u32 = 64 - SCOPE_BITS;

/// The last sequence number an issuer hands out before it is [`Refusal::Exhausted`].
pub const SEQUENCE_MAX: u64 = (1 << SEQUENCE_BITS) - 1;

/// Build a raw id from a scope and a sequence number.
///
/// Sequence bits above [`SEQUENCE_MAX`] are dropped, so the scope is always
/// the one given.
#[must_use]
pub const fn scoped(scope: u16, sequence: u64) -> u64 {
    ((scope as u64) << SEQUENCE_BITS) | (sequence & SEQUENCE_MAX)
}

/// The scope a raw id was issued under.
#[must_use]
pub const fn scope_of(raw: u64) -> u16 {
    (raw >> SEQUENCE_BITS) as u16
}

/// The sequence number inside a raw id. Zero for an id no issuer ever issued.
#[must_use]
pub const fn sequence_of(raw: u64) -> u64 {
    raw & SEQUENCE_MAX
}

/// Why an [`Issuer`] issued nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// All [`SEQUENCE_MAX`] ids of this scope have been issued.
    Exhausted,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Exhausted => "every id of this scope has been issued",
        })
    }
}

impl core::error::Error for Refusal {}

/// Issues the ids of one scope, in order, each exactly once.
///
/// Sequence numbers start at one, so a raw id of zero is never issued and can
/// stand for "no id" in a `Default`. The issuer never reads a clock or a
/// random source: two issuers given the same scope issue the same ids, and
/// keeping scopes distinct is the caller's job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Issuer {
    scope: u16,
    issued: u64,
}

impl Default for Issuer {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Issuer {
    /// Start a scope with nothing issued.
    #[must_use]
    pub const fn new(scope: u16) -> Self {
        Self { scope, issued: 0 }
    }

    /// Resume a scope that has already issued `issued` ids, from saved state.
    ///
    /// The next id is sequence `issued + 1`; everything up to `issued` is owned.
    /// `None` when `issued` is beyond [`SEQUENCE_MAX`], which no issuer reaches.
    /// Resuming from a count lower than what was really issued reissues ids:
    /// persist [`Issuer::issued`] before handing an id out.
    #[must_use]
    pub const fn resumed(scope: u16, issued: u64) -> Option<Self> {
        if issued > SEQUENCE_MAX {
            return None;
        }
        Some(Self { scope, issued })
    }

    /// The scope carried by every id this issuer hands out.
    #[must_use]
    pub const fn scope(&self) -> u16 {
        self.scope
    }

    /// How many ids have been issued; also the sequence number of the last one.
    #[must_use]
    pub const fn issued(&self) -> u64 {
        self.issued
    }

    /// How many ids are left before [`Refusal::Exhausted`].
    #[must_use]
    pub const fn remaining(&self) -> u64 {
        SEQUENCE_MAX - self.issued
    }

    /// The id the next [`Issuer::issue`] will return, without issuing it.
    ///
    /// For callers that must check their own capacity before committing: peek,
    /// decide, then issue.
    ///
    /// # Errors
    ///
    /// [`Refusal::Exhausted`] when the scope is spent.
    pub const fn peek(&self) -> Result<u64, Refusal> {
        if self.issued == SEQUENCE_MAX {
            return Err(Refusal::Exhausted);
        }
        Ok(scoped(self.scope, self.issued + 1))
    }

    /// Issue the next id of this scope.
    ///
    /// # Errors
    ///
    /// [`Refusal::Exhausted`] when the scope is spent. Nothing changes.
    pub const fn issue(&mut self) -> Result<u64, Refusal> {
        match self.peek() {
            Ok(id) => {
                self.issued += 1;
                Ok(id)
            }
            Err(refusal) => Err(refusal),
        }
    }

    /// Whether `raw` is an id this issuer has handed out.
    ///
    /// True when the scope matches and the sequence number is one that was
    /// issued. It says nothing about whether the id is still live: that is the
    /// caller's table. An id from another scope is never owned.
    #[must_use]
    pub const fn owns(&self, raw: u64) -> bool {
        let sequence = sequence_of(raw);
        scope_of(raw) == self.scope && sequence != 0 && sequence <= self.issued
    }
}

#[cfg(test)]
mod tests;
