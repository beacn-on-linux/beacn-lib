use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

#[derive(Copy, Clone, Default, Hash, PartialEq, Eq)]
pub struct VersionNumber(pub u32, pub u32, pub u32, pub u32);

impl PartialOrd for VersionNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VersionNumber {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.0.cmp(&other.0) {
            Ordering::Greater => return Ordering::Greater,
            Ordering::Less => return Ordering::Less,
            Ordering::Equal => {}
        }

        match self.1.cmp(&other.1) {
            Ordering::Greater => return Ordering::Greater,
            Ordering::Less => return Ordering::Less,
            Ordering::Equal => {}
        }

        match self.2.cmp(&other.2) {
            Ordering::Greater => return Ordering::Greater,
            Ordering::Less => return Ordering::Less,
            Ordering::Equal => {}
        }

        match self.3.cmp(&other.3) {
            Ordering::Greater => return Ordering::Greater,
            Ordering::Less => return Ordering::Less,
            Ordering::Equal => {}
        }

        Ordering::Equal
    }
}

impl Display for VersionNumber {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0, self.1, self.2, self.3)
    }
}

impl std::fmt::Debug for VersionNumber {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl From<String> for VersionNumber {
    fn from(value: String) -> Self {
        let mut version = VersionNumber::default();

        let mut parts = value.split('.');

        // We can't iterate over a tuple, so we need to do this 4 times..
        if let Some(part) = parts.next() {
            if let Ok(part) = part.parse() {
                version.0 = part;
            }
        }

        if let Some(part) = parts.next() {
            if let Ok(part) = part.parse() {
                version.1 = part;
            }
        }

        if let Some(part) = parts.next() {
            if let Ok(part) = part.parse() {
                version.2 = part;
            }
        }

        if let Some(part) = parts.next() {
            if let Ok(part) = part.parse() {
                version.3 = part;
            }
        }

        version
    }
}
