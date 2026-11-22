use std::fmt;

/// Type of tensor index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexType {
    /// Covariant index (subscript), e.g., $T_i$
    Covariant,
    /// Contravariant index (superscript), e.g., $T^i$
    Contravariant,
}

/// A tensor index (Ricci calculus) for Einstein summation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Index {
    /// The name/symbol of the index (e.g., "i", "j", "mu")
    pub name: String,
    /// Whether it is covariant or contravariant
    pub idx_type: IndexType,
}

impl Index {
    /// Create a covariant index (lower).
    pub fn cov(name: &str) -> Self {
        Self {
            name: name.to_string(),
            idx_type: IndexType::Covariant,
        }
    }

    /// Create a contravariant index (upper).
    pub fn con(name: &str) -> Self {
        Self {
            name: name.to_string(),
            idx_type: IndexType::Contravariant,
        }
    }
}

impl fmt::Display for Index {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.idx_type {
            IndexType::Covariant => write!(f, "_{}", self.name),
            IndexType::Contravariant => write!(f, "^{}", self.name),
        }
    }
}

/// Encode a tensor into a string symbol for the e-graph.
pub fn encode_tensor_symbol(name: &str, indices: &[Index]) -> String {
    let mut s = format!("T|{}", name);
    for idx in indices {
        s.push_str(match idx.idx_type {
            IndexType::Covariant => ",C|",
            IndexType::Contravariant => ",U|",
        });
        s.push_str(&idx.name);
    }
    s
}

/// Decode a tensor from a string symbol from the e-graph.
pub fn decode_tensor_symbol(s: &str) -> Option<(String, Vec<Index>)> {
    if !s.starts_with("T|") {
        return None;
    }
    let mut parts = s[2..].split(',');
    let name = parts.next()?.to_string();
    let mut indices = Vec::new();
    for part in parts {
        if let Some(stripped) = part.strip_prefix("C|") {
            indices.push(Index::cov(stripped));
        } else {
            let stripped = part.strip_prefix("U|")?;
            indices.push(Index::con(stripped));
        }
    }
    Some((name, indices))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_tensor_symbol() {
        let name = "T".to_string();
        let indices = vec![Index::con("mu"), Index::cov("nu")];
        let encoded = encode_tensor_symbol(&name, &indices);
        assert_eq!(encoded, "T|T,U|mu,C|nu");

        let decoded = decode_tensor_symbol(&encoded).unwrap();
        assert_eq!(decoded.0, "T");
        assert_eq!(decoded.1, indices);
    }
}
