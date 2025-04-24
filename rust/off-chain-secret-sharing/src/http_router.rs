use crate::{ContractState, JSON_RESPONSE_UNKNOWN_METHOD, JSON_RESPONSE_UNKNOWN_URL};
use matchit::{Params, Router};
use pbc_contract_common::off_chain::{HttpRequestData, HttpResponseData, OffChainContext};
use std::collections::BTreeMap;

/// Type of functions that can be dispatched to.
///
/// Matches the type of the `off_chain_on_http_request` with HTTP [`Params`].
type DispatchFunction = fn(
    OffChainContext,
    ContractState,
    HttpRequestData,
    Params,
) -> Result<HttpResponseData, HttpResponseData>;

/// Http router to route incoming http requests to its corresponding function
pub struct HttpRouter {
    /// Matchable routes. The key is the HTTP path, and the value is the list of
    /// HTTP methods to be found at that path.
    routes: BTreeMap<String, Vec<HttpMethod>>,
}

impl HttpRouter {
    /// Create a new router
    pub fn new() -> HttpRouter {
        HttpRouter {
            routes: BTreeMap::new(),
        }
    }

    /// Insert a new route to a function
    ///
    /// # Arguments
    ///
    /// * `route` - The route where the method is called
    /// * `method` - The http method and function to call
    pub fn insert(&mut self, route: &str, method: HttpMethod) {
        let vec = self.routes.entry(route.into()).or_default();
        vec.push(method);
    }

    /// Dispatch the http request through the router
    ///
    /// # Arguments
    ///
    /// * `ctx` - the off chain context for accessing external systems
    /// * `state` - the contract state
    /// * `request` - the received http request
    pub fn dispatch(
        self,
        ctx: OffChainContext,
        state: ContractState,
        request: HttpRequestData,
    ) -> Result<HttpResponseData, HttpResponseData> {
        let mut router: Router<Vec<HttpMethod>> = Router::new();
        for (route, methods) in self.routes {
            router.insert(&route, methods).unwrap();
        }

        let uri = request.uri.clone();
        let routed = router
            .at(&uri)
            .map_err(|_| HttpResponseData::new_with_str(404, JSON_RESPONSE_UNKNOWN_URL))?;

        let methods = routed.value;

        let dispatch = methods
            .iter()
            .find(|method| method.method_type() == request.method.as_str().to_lowercase())
            .ok_or(HttpResponseData::new_with_str(
                405,
                JSON_RESPONSE_UNKNOWN_METHOD,
            ))?
            .get_function();

        dispatch(ctx, state, request, routed.params)
    }
}

/// Http method that can be called by the router
pub enum HttpMethod {
    /// Get method
    Get(DispatchFunction),
    /// Put method
    Put(DispatchFunction),
}

impl HttpMethod {
    /// Get the method type as a string
    pub fn method_type(&self) -> &str {
        match self {
            HttpMethod::Get(_) => "get",
            HttpMethod::Put(_) => "put",
        }
    }

    /// Get the rust function of this http method
    pub fn get_function(&self) -> &DispatchFunction {
        match self {
            HttpMethod::Get(function) => function,
            HttpMethod::Put(function) => function,
        }
    }
}
