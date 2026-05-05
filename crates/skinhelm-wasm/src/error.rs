#[derive(Debug, Clone)]
pub enum ViewerError {
    MissingElement(&'static str),
    WebGlUnavailable,
    ShaderCompile(String),
    ProgramLink(String),
    BufferCreation,
    TextureCreation,
    InvalidPng,
    InvalidCapePng,
    UnsupportedSkinDimensions(u32, u32),
    UnsupportedCapeDimensions(u32, u32),
    WebGlOperation(&'static str),
}

impl ViewerError {
    pub fn message(&self) -> String {
        match self {
            Self::MissingElement(id) => format!("missing DOM element: {id}"),
            Self::WebGlUnavailable => "WebGL2 is not available".to_owned(),
            Self::ShaderCompile(message) => format!("shader compile failed: {message}"),
            Self::ProgramLink(message) => format!("program link failed: {message}"),
            Self::BufferCreation => "failed to create WebGL buffer".to_owned(),
            Self::TextureCreation => "failed to create WebGL texture".to_owned(),
            Self::InvalidPng => "invalid PNG skin".to_owned(),
            Self::InvalidCapePng => "invalid PNG cape".to_owned(),
            Self::UnsupportedSkinDimensions(width, height) => {
                format!("unsupported skin size: {width}x{height}")
            }
            Self::UnsupportedCapeDimensions(width, height) => {
                format!("unsupported cape size: {width}x{height}")
            }
            Self::WebGlOperation(operation) => format!("WebGL operation failed: {operation}"),
        }
    }
}

impl std::fmt::Display for ViewerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message())
    }
}
