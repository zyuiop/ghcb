use crate::protocols::GhcbProtocolRequest;
use crate::structures::channel::GhcbRequestExecutor;
use crate::structures::exit_codes::GhcbExitCode;
use crate::structures::ghcb_page::GhcbU64Field;
use crate::structures::snp_guest_request::error::GuestProtocolError;
use crate::structures::snp_guest_request::shared_page::{SNPSharedPage, SharedPageAccessor};
use crate::structures::snp_guest_request::SNPGuestRequest;
use crate::structures::snp_secrets_page::SecretsPageAccessor;
use crate::util::OwnedPtrWithPhysAddr;

pub struct SnpGuestRequest<'a, R: SNPGuestRequest, SP: SecretsPageAccessor, T: SharedPageAccessor> {
    request: R,
    secrets_accessor: &'a SP,
    request_page: &'a T,
    response_page: &'a T
}

impl<'a, R: SNPGuestRequest, SP: SecretsPageAccessor, T: SharedPageAccessor> SnpGuestRequest<'a, R, SP, T> {
    pub fn new(request: R, secrets_accessor: &'a SP, request_page: &'a T, response_page: &'a T) -> Self {
        Self {
            request, secrets_accessor, request_page, response_page
        }
    }
}

impl<R: SNPGuestRequest, SP: SecretsPageAccessor, T: SharedPageAccessor> GhcbProtocolRequest for SnpGuestRequest<'_, R, SP, T> {
    type Response = Result<R::ResponseType, GuestProtocolError>;

    fn execute_request(self, ghcb: &mut GhcbRequestExecutor) -> Self::Response {
        self.request_page.with_shared_page(|req_page| {
            req_page.write_request(self.secrets_accessor, self.request);

            self.response_page.with_shared_page(|resp_page| {
                resp_page.clear();

                ghcb.checked_vmgexit(
                    GhcbExitCode::SnpGuestRequest,
                    req_page.phys_addr().as_u64(),
                    resp_page.phys_addr().as_u64(),
                );

                let exit2 = ghcb.raw()
                    .get_field_if_valid(GhcbU64Field::SwExitInfo2)
                    .expect("missing SWExitInfo2 field");

                if exit2 != 0 {
                    return Err(GuestProtocolError::from_fw_error(exit2));
                }

                let decrypted: R::ResponseType = resp_page.read_response(self.secrets_accessor);
                Ok(decrypted)
            })
        })
    }
}