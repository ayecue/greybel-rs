//! File types and the commands files launch.

enum_def! {
    /// File type (`Fichero.TypeFile`), with `Generic` split into text and binary by
    /// `Archivo.isBinario`. Code 0 denotes a folder.
    pub enum FileKind {
        Text = 1,
        Binary = 2,
        /// Compiled program with retained source, launched by the interpreter (`TypeFile.Script`).
        Program = 3,
        SystemLog = 4,
        Library = 5,
        ChatLog = 6,
        Image = 7,
        Pdf = 8,
        BankLog = 9,
    }
}

impl FileKind {
    /// Maps `Fichero.TypeFile` number `file_type` and `Archivo.isBinario` flag `binary` to a file kind.
    #[must_use]
    pub const fn of_layout(file_type: u8, binary: bool) -> Self {
        match file_type {
            1 => Self::Program,
            2 => Self::SystemLog,
            3 => Self::Library,
            4 => Self::ChatLog,
            5 => Self::Image,
            6 => Self::Pdf,
            7 => Self::BankLog,
            _ if binary => Self::Binary,
            _ => Self::Text,
        }
    }

    /// Checks whether this file kind is binary and cannot be read as text.
    #[must_use]
    pub const fn is_binary(self) -> bool {
        !matches!(self, Self::Text | Self::ChatLog | Self::BankLog)
    }
}

enum_def! {
    /// Application launched by a binary (`Archivo.comando`).
    pub enum Command {
        FileExplorer = 0,
        Terminal = 1,
        Mail = 2,
        Browser = 3,
        Notepad = 4,
        Settings = 5,
        LogViewer = 6,
        CodeEditor = 7,
        ImageViewer = 8,
        PdfReader = 9,
        Map = 10,
        Manual = 11,
        ScanLan = 12,
        Chat = 13,
        Nmap = 14,
    }
}

impl Command {
    /// Returns the executable and process name for this command.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::FileExplorer => "FileExplorer.exe",
            Self::Terminal => "Terminal.exe",
            Self::Mail => "Mail.exe",
            Self::Browser => "Browser.exe",
            Self::Notepad => "Notepad.exe",
            Self::Settings => "Settings.exe",
            Self::LogViewer => "LogViewer.exe",
            Self::CodeEditor => "CodeEditor.exe",
            Self::ImageViewer => "ImageViewer.exe",
            Self::PdfReader => "PDFReader.exe",
            Self::Map => "Map.exe",
            Self::Manual => "Manual.exe",
            Self::ScanLan => "ScanLan.exe",
            Self::Chat => "Chat.exe",
            Self::Nmap => "nmap",
        }
    }

    /// Looks up the command with executable name `name`.
    #[must_use]
    pub fn of_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|command| command.name() == name)
    }
}
