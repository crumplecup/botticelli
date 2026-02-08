//! Media storage MCP tools.
//!
//! Auto-generated from MediaStorage trait via #[elicit_trait_tools_router] macro.

use botticelli_interface::MediaStorage;
use botticelli_storage::{MediaMetadata, MediaReference};
use elicitation_macros::elicit_trait_tools_router;
use rmcp::tool_router;

// Type aliases for FileSystemStorage's associated types
type StoreParams = botticelli_interface::StoreParams<MediaMetadata, Vec<u8>>;
type StoreResult = botticelli_interface::StoreResult<MediaReference>;
type RetrieveParams = botticelli_interface::RetrieveParams<MediaReference>;
type RetrieveResult = botticelli_interface::RetrieveResult;
type GetUrlParams = botticelli_interface::GetUrlParams<MediaReference>;
type GetUrlResult = botticelli_interface::GetUrlResult;
type DeleteParams = botticelli_interface::DeleteParams<MediaReference>;
type DeleteResult = botticelli_interface::DeleteResult;
type ExistsParams = botticelli_interface::ExistsParams<MediaReference>;
type ExistsResult = botticelli_interface::ExistsResult;

/// Storage tool implementations for the MCP server.
///
/// Generated tool router function: `storage_tool_router()`
///
/// Note: The #[tool_router] macro generates the `storage_tool_router()` function
/// without documentation attributes. Since we cannot modify the macro-generated
/// code, we use #[allow(missing_docs)] as an explicit exception to the crate's
/// missing_docs lint. This is acceptable for macro-generated code where
/// documentation would need to be added by the macro itself (upstream fix).
#[allow(missing_docs)]
#[elicit_trait_tools_router(MediaStorage, storage, [store, retrieve, get_url, delete, exists])]
#[tool_router(router = storage_tool_router, vis = "pub")]
impl crate::BotticelliServer {}
