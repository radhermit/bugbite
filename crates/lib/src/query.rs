use std::fmt;
use std::str::FromStr;

use crate::Error;

#[derive(Debug, Clone, Copy)]
pub(crate) enum OrderType {
    Ascending,
    Descending,
}

/// Invertable search order sorting term.
#[derive(Debug, Clone, Copy)]
pub struct Order<T> {
    pub(crate) order: OrderType,
    pub(crate) field: T,
}

impl<T: FromStr> TryFrom<&str> for Order<T> {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl<T: FromStr> FromStr for Order<T> {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        let (order, field) = if let Some(value) = s.strip_prefix('-') {
            (OrderType::Descending, value)
        } else {
            (OrderType::Ascending, s.strip_prefix('+').unwrap_or(s))
        };
        let field = field
            .parse()
            .map_err(|_| Error::InvalidValue(format!("unknown search field: {field}")))?;
        Ok(Self { order, field })
    }
}

impl<T: fmt::Display> fmt::Display for Order<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.order {
            OrderType::Descending => write!(f, "-{}", self.field),
            OrderType::Ascending => write!(f, "{}", self.field),
        }
    }
}
