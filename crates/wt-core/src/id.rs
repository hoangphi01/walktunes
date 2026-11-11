// SPDX-License-Identifier: MIT OR Apache-2.0
//! Typed identifiers. IDs never cross crate boundaries as bare `Uuid` or `String`
//! (STYLE_RULE §2). New IDs are UUIDv7, so they sort by creation time.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Creates a new, time-ordered (UUIDv7) identifier.
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            /// Wraps an existing UUID (for example, one read from storage).
            #[must_use]
            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Returns the underlying UUID.
            #[must_use]
            pub const fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(s.trim()).map(Self)
            }
        }
    };
}

define_id!(
    /// A logical recording, independent of files.
    TrackId
);
define_id!(
    /// A physical audio file inside a library root.
    FileId
);
define_id!(
    /// A folder registered in the library.
    RootId
);
define_id!(
    /// A paired player (stored in `device.json`).
    DeviceId
);
define_id!(
    /// One computer running WalkTunes.
    MachineId
);
define_id!(
    /// A background job.
    JobId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip_through_strings() {
        let id = DeviceId::new();
        let parsed: DeviceId = id.to_string().parse().unwrap_or_default();
        assert_eq!(id, parsed);
    }

    #[test]
    fn new_ids_are_time_ordered() {
        let a = JobId::new();
        let b = JobId::new();
        assert!(a <= b);
    }

    #[test]
    fn ids_serialize_as_plain_strings() {
        let id = MachineId::new();
        let json = serde_json::to_string(&id).unwrap_or_default();
        assert_eq!(json, format!("\"{id}\""));
    }
}
