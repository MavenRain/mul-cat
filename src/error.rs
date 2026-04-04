//! Single project-wide error type.

use comp_cat_rs::collapse::free_category::FreeCategoryError;

/// All errors produced by the `mul-cat` crate.
#[derive(Debug)]
pub enum Error {
    /// Graph construction or traversal error from comp-cat-rs.
    Graph(FreeCategoryError),

    /// Topology level index out of bounds.
    LevelOutOfBounds {
        /// The requested level.
        level: usize,
        /// The total number of levels.
        count: usize,
    },

    /// A reduction topology produced a term index that does not
    /// exist at the given level.
    TermIndexOutOfRange {
        /// The level where the invalid reference occurred.
        level: usize,
        /// The invalid term index.
        index: usize,
        /// The number of terms available at this level.
        available: usize,
    },

    /// A reduction topology produced a grouping that does not
    /// preserve the invariant `2 * triples + passthroughs == input_count`.
    GroupingMismatch {
        /// The number of input terms the grouping was applied to.
        input_count: usize,
        /// The number of triples produced.
        triples: usize,
        /// The number of passthroughs produced.
        passthroughs: usize,
    },

    /// Bit width must be positive.
    ZeroBitWidth,

    /// Bit width exceeds what the `u128` internal representation can hold.
    ///
    /// For the Booth multiplier, the product has width `2 * N`, so the
    /// operand width is limited to `64`.
    BitWidthTooLarge {
        /// The requested operand bit width.
        width: usize,
        /// The maximum supported operand bit width.
        max: usize,
    },

    /// Schoolbook multiply called with differently sized coefficient arrays.
    CoefficientCountMismatch {
        /// Number of coefficients in `a`.
        a_count: usize,
        /// Number of coefficients in `b`.
        b_count: usize,
    },

    /// Schoolbook multiply called with zero coefficients.
    ZeroCoefficientCount,

    /// Word length exceeds the coefficient product width.
    WordLengthTooLarge {
        /// The requested word length.
        word_len: usize,
        /// The coefficient product width.
        product_width: usize,
    },
}

impl From<FreeCategoryError> for Error {
    fn from(e: FreeCategoryError) -> Self {
        Self::Graph(e)
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Graph(e) => write!(f, "graph error: {e}"),
            Self::LevelOutOfBounds { level, count } => {
                write!(f, "level {level} out of bounds (count: {count})")
            }
            Self::TermIndexOutOfRange {
                level,
                index,
                available,
            } => write!(
                f,
                "term index {index} out of range at level {level} (available: {available})"
            ),
            Self::GroupingMismatch {
                input_count,
                triples,
                passthroughs,
            } => write!(
                f,
                "grouping mismatch: {input_count} inputs != {triples} triples * 3 + {passthroughs} passthroughs"
            ),
            Self::ZeroBitWidth => write!(f, "bit width must be positive"),
            Self::BitWidthTooLarge { width, max } => {
                write!(f, "bit width {width} exceeds maximum {max}")
            }
            Self::CoefficientCountMismatch { a_count, b_count } => write!(
                f,
                "coefficient count mismatch: a has {a_count}, b has {b_count}"
            ),
            Self::ZeroCoefficientCount => write!(f, "coefficient count must be positive"),
            Self::WordLengthTooLarge {
                word_len,
                product_width,
            } => write!(
                f,
                "word length {word_len} exceeds coefficient product width {product_width}"
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Graph(e) => Some(e),
            Self::LevelOutOfBounds { .. }
            | Self::TermIndexOutOfRange { .. }
            | Self::GroupingMismatch { .. }
            | Self::ZeroBitWidth
            | Self::BitWidthTooLarge { .. }
            | Self::CoefficientCountMismatch { .. }
            | Self::ZeroCoefficientCount
            | Self::WordLengthTooLarge { .. } => None,
        }
    }
}
