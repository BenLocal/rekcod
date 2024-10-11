use serde::{Deserialize, Serialize};

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Default)]
pub struct ApiJsonResponse<T>
where
    T: Sized + Serialize + Send + Sync,
{
    msg: String,
    code: i32,
    data: Option<T>,
}

#[allow(dead_code)]
impl<T> ApiJsonResponse<T>
where
    T: Sized + Serialize + Send + Sync,
{
    pub fn empty_success() -> Self {
        Self {
            msg: "".to_string(),
            code: 0,
            data: None::<T>,
        }
    }

    pub fn success(d: T) -> Self {
        Self {
            msg: "".to_string(),
            code: 0,
            data: Some(d),
        }
    }

    pub fn success_optional(d: Option<T>) -> Self {
        Self {
            msg: "".to_string(),
            code: 0,
            data: d,
        }
    }

    pub fn empty_error(code: i32, msg: &str) -> Self {
        Self {
            msg: msg.to_string(),
            code: code,
            data: None::<T>,
        }
    }
}
