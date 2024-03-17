use crate::traits::ServiceParams;

pub struct CreateParams<'a> {
    _service: &'a super::Service,
}

impl<'a> ServiceParams<'a> for CreateParams<'a> {
    type Service = super::Service;

    fn new(service: &'a Self::Service) -> Self {
        Self { _service: service }
    }
}
