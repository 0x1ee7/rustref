//! Part 5: `routes!`, a declarative route table validated at construction.

use std::fmt;

/// HTTP method supported by the router. Ordering follows declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl Method {
    /// Canonical upper-case name, as sent on the wire.
    pub const fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
            Method::Delete => "DELETE",
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One route: method, pattern, and handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route<H> {
    pub method: Method,
    pub pattern: &'static str,
    pub handler: H,
}

impl<H> Route<H> {
    /// Builds a route. Pattern validation happens in [`Router::new`].
    pub const fn new(method: Method, pattern: &'static str, handler: H) -> Self {
        Self {
            method,
            pattern,
            handler,
        }
    }
}

/// Why a route table was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteError {
    /// Two routes share a method and a shape.
    Duplicate {
        method: Method,
        pattern: &'static str,
        existing: &'static str,
    },
    /// The pattern is malformed.
    InvalidPattern {
        pattern: &'static str,
        reason: &'static str,
    },
}

impl fmt::Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RouteError::Duplicate {
                method,
                pattern,
                existing,
            } => write!(
                f,
                "duplicate route {method} `{pattern}` conflicts with `{existing}`"
            ),
            RouteError::InvalidPattern { pattern, reason } => {
                write!(f, "invalid route pattern `{pattern}`: {reason}")
            }
        }
    }
}

impl std::error::Error for RouteError {}

/// One parsed pattern segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Segment {
    Static(&'static str),
    Param(&'static str),
}

#[derive(Debug)]
struct Compiled<H> {
    method: Method,
    pattern: &'static str,
    segments: Vec<Segment>,
    handler: H,
}

/// A validated route table.
#[derive(Debug)]
pub struct Router<H> {
    routes: Vec<Compiled<H>>,
}

impl<H> Router<H> {
    /// Validates every pattern and rejects duplicate method+shape pairs.
    ///
    /// Routes are checked in table order; the first offending route is reported.
    pub fn new(routes: Vec<Route<H>>) -> Result<Self, RouteError> {
        let mut compiled: Vec<Compiled<H>> = Vec::with_capacity(routes.len());
        for Route {
            method,
            pattern,
            handler,
        } in routes
        {
            let segments = parse_pattern(pattern)?;
            if let Some(existing) = compiled
                .iter()
                .find(|c| c.method == method && same_shape(&c.segments, &segments))
            {
                return Err(RouteError::Duplicate {
                    method,
                    pattern,
                    existing: existing.pattern,
                });
            }
            compiled.push(Compiled {
                method,
                pattern,
                segments,
                handler,
            });
        }
        Ok(Self { routes: compiled })
    }

    /// Number of routes.
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// Whether the table has no routes.
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    /// Resolves `method` and `path`. See the module docs for the rules.
    pub fn lookup<'r, 'p>(&'r self, method: Method, path: &'p str) -> Lookup<'r, 'p, H> {
        let Some(rest) = path.strip_prefix('/') else {
            return Lookup::NotFound;
        };
        let parts: Vec<&'p str> = if rest.is_empty() {
            Vec::new()
        } else {
            rest.split('/').collect()
        };

        type Best<'a, 'b, T> = (Vec<u8>, &'a Compiled<T>, Vec<(&'static str, &'b str)>);
        let mut best: Option<Best<'r, 'p, H>> = None;
        let mut allowed: Vec<Method> = Vec::new();

        for route in &self.routes {
            let Some(params) = match_segments(&route.segments, &parts) else {
                continue;
            };
            if route.method != method {
                allowed.push(route.method);
                continue;
            }
            // Lower rank wins: a literal (0) beats a parameter (1) at the first
            // segment where they differ. Equal-length patterns compare lexicographically.
            let rank: Vec<u8> = route
                .segments
                .iter()
                .map(|s| u8::from(matches!(s, Segment::Param(_))))
                .collect();
            if best.as_ref().is_none_or(|(b, _, _)| rank < *b) {
                best = Some((rank, route, params));
            }
        }

        match best {
            Some((_, route, params)) => Lookup::Found(Match {
                handler: &route.handler,
                params,
            }),
            None if allowed.is_empty() => Lookup::NotFound,
            None => {
                allowed.sort();
                allowed.dedup();
                Lookup::MethodNotAllowed(allowed)
            }
        }
    }
}

/// Splits and validates a pattern.
fn parse_pattern(pattern: &'static str) -> Result<Vec<Segment>, RouteError> {
    let invalid = |reason: &'static str| RouteError::InvalidPattern { pattern, reason };

    let Some(rest) = pattern.strip_prefix('/') else {
        return Err(invalid("pattern must start with `/`"));
    };
    if rest.is_empty() {
        return Ok(Vec::new());
    }

    let mut segments = Vec::new();
    let mut names: Vec<&'static str> = Vec::new();
    for raw in rest.split('/') {
        if raw.is_empty() {
            return Err(invalid("empty path segment"));
        }
        match raw.strip_prefix(':') {
            Some("") => return Err(invalid("parameter name is empty")),
            Some(name) => {
                if names.contains(&name) {
                    return Err(invalid("duplicate parameter name"));
                }
                names.push(name);
                segments.push(Segment::Param(name));
            }
            None => segments.push(Segment::Static(raw)),
        }
    }
    Ok(segments)
}

/// Two patterns have the same shape when they agree on literals and parameter positions.
fn same_shape(a: &[Segment], b: &[Segment]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|pair| {
            matches!(
                pair,
                (Segment::Static(x), Segment::Static(y)) if x == y
            ) || matches!(pair, (Segment::Param(_), Segment::Param(_)))
        })
}

/// Matches `parts` against `segments`, returning captured parameters in pattern order.
fn match_segments<'p>(
    segments: &[Segment],
    parts: &[&'p str],
) -> Option<Vec<(&'static str, &'p str)>> {
    if segments.len() != parts.len() {
        return None;
    }
    let mut params = Vec::new();
    for (segment, &part) in segments.iter().zip(parts) {
        match *segment {
            Segment::Static(text) if text == part => {}
            Segment::Static(_) => return None,
            Segment::Param(_) if part.is_empty() => return None,
            Segment::Param(name) => params.push((name, part)),
        }
    }
    Some(params)
}

/// Outcome of [`Router::lookup`].
#[derive(Debug)]
pub enum Lookup<'r, 'p, H> {
    /// A route matched.
    Found(Match<'r, 'p, H>),
    /// The path exists under other methods; these are the allowed ones, sorted.
    MethodNotAllowed(Vec<Method>),
    /// No route matches the path.
    NotFound,
}

/// A successful match: the handler plus captured parameters.
#[derive(Debug)]
pub struct Match<'r, 'p, H> {
    pub handler: &'r H,
    /// `(name, value)` for every `:name` segment, in pattern order.
    pub params: Vec<(&'static str, &'p str)>,
}

impl<'p, H> Match<'_, 'p, H> {
    /// Captured value of `:name`, if the pattern has that parameter.
    pub fn param(&self, name: &str) -> Option<&'p str> {
        self.params
            .iter()
            .find(|(n, _)| *n == name)
            .map(|&(_, value)| value)
    }
}

/// Builds a [`Router`] from a table of `METHOD "pattern" => handler` lines.
///
/// Recursively munches the table into a `Vec<Route<_>>`, mapping each method
/// identifier to a [`Method`] variant. Evaluates to `Result<Router<H>, RouteError>`.
#[macro_export]
macro_rules! routes {
    (@method GET) => {
        $crate::Method::Get
    };
    (@method POST) => {
        $crate::Method::Post
    };
    (@method PUT) => {
        $crate::Method::Put
    };
    (@method PATCH) => {
        $crate::Method::Patch
    };
    (@method DELETE) => {
        $crate::Method::Delete
    };
    (@method $other:ident) => {
        ::core::compile_error!(::core::concat!(
            "routes!: unknown method `",
            ::core::stringify!($other),
            "`"
        ))
    };
    (@acc [$($route:expr,)*]) => {
        $crate::Router::new(::std::vec![$($route,)*])
    };
    (@acc [$($route:expr,)*] $method:ident $pattern:literal => $handler:expr, $($rest:tt)*) => {
        $crate::routes!(@acc [
            $($route,)*
            $crate::Route::new($crate::routes!(@method $method), $pattern, $handler),
        ] $($rest)*)
    };
    (@acc [$($route:expr,)*] $method:ident $pattern:literal => $handler:expr) => {
        $crate::routes!(@acc [$($route,)*] $method $pattern => $handler,)
    };
    ($($input:tt)*) => {
        $crate::routes!(@acc [] $($input)*)
    };
}
