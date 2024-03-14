use crate::traits::ServiceParams;

pub struct ModifyParams<'a> {
    _service: &'a super::Service,
}

impl<'a> ServiceParams<'a> for ModifyParams<'a> {
    type Service = super::Service;

    fn new(service: &'a Self::Service) -> Self {
        Self { _service: service }
    }
}
