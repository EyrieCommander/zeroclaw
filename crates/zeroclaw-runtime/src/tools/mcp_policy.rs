use std::sync::Arc;

use zeroclaw_api::tool::Tool;
use zeroclaw_tools::tool_search::ToolAccessPolicy;

use super::{ArcToolRef, DelegateParentToolsHandle};

pub fn mcp_tool_access_policy(
    security: &zeroclaw_config::policy::SecurityPolicy,
    caller_allowed: Option<&[String]>,
) -> Option<ToolAccessPolicy> {
    ToolAccessPolicy::from_security(
        security.allowed_tools.as_deref(),
        security.excluded_tools.as_deref(),
        caller_allowed,
    )
}

pub fn eager_mcp_tool_allowed(name: &str, policy: Option<&ToolAccessPolicy>) -> bool {
    policy.is_none_or(|policy| policy.is_tool_allowed(name))
}

pub fn mcp_allowed_tool_count<'a>(
    names: impl IntoIterator<Item = &'a str>,
    policy: Option<&ToolAccessPolicy>,
) -> usize {
    names
        .into_iter()
        .filter(|name| eager_mcp_tool_allowed(name, policy))
        .count()
}

pub fn register_eager_mcp_tool_if_allowed(
    wrapper: Arc<dyn Tool>,
    tools: &mut Vec<Box<dyn Tool>>,
    delegate_handle: Option<&DelegateParentToolsHandle>,
    policy: Option<&ToolAccessPolicy>,
) -> bool {
    if !eager_mcp_tool_allowed(wrapper.name(), policy) {
        return false;
    }
    if let Some(handle) = delegate_handle {
        handle.write().push(Arc::clone(&wrapper));
    }
    tools.push(Box::new(ArcToolRef(wrapper)));
    true
}
