pub mod deps;
pub mod docx_outline;
pub mod export;
pub mod fs_commands;
pub mod import;
pub mod mindmap;
pub mod pdf_import;
/// Harness de ida-e-volta `MD → PDF → MD` — só existe em teste (ver o módulo).
#[cfg(test)]
mod pdf_roundtrip;
pub mod startup;
pub mod templates;
pub mod win_assoc;
