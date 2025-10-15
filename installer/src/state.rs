use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallerState {
    pub current_step: usize,
    pub locale: LocaleSelection,
    pub keyboard: KeyboardSelection,
    pub timezone: Option<String>,
    pub target: Option<DiskPlan>,
    pub users: Vec<UserAccount>,
    pub network: NetworkConfig,
    pub discovered_disks: Vec<DiskIdentifier>,
    pub selected_disk: Option<DiskIdentifier>,
}

impl Default for InstallerState {
    fn default() -> Self {
        Self {
            current_step: 0,
            locale: LocaleSelection::default(),
            keyboard: KeyboardSelection::default(),
            timezone: None,
            target: None,
            users: vec![UserAccount::default_admin()],
            network: NetworkConfig::default(),
            discovered_disks: Vec::new(),
            selected_disk: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleSelection {
    pub language: String,
    pub region: String,
}

impl Default for LocaleSelection {
    fn default() -> Self {
        Self {
            language: "en".into(),
            region: "US".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardSelection {
    pub layouts: Vec<String>,
    pub variant: Option<String>,
}

impl Default for KeyboardSelection {
    fn default() -> Self {
        Self {
            layouts: vec!["us".into()],
            variant: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskPlan {
    pub target: DiskIdentifier,
    pub mode: DiskMode,
    pub partitions: Vec<PartitionSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiskIdentifier {
    pub path: String,
    pub size_bytes: u64,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiskMode {
    UseEntireDisk,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionSpec {
    pub id: String,
    pub mountpoint: Option<String>,
    pub filesystem: FileSystem,
    pub size: PartitionSize,
    pub flags: Vec<PartitionFlag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PartitionSize {
    ExactBytes(u64),
    Percentage(u8),
    Remainder,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileSystem {
    Ext4,
    Btrfs,
    Xfs,
    Swap,
    Fat32,
    Other(String),
}

impl FileSystem {
    pub fn label(&self) -> &str {
        match self {
            FileSystem::Ext4 => "ext4",
            FileSystem::Btrfs => "btrfs",
            FileSystem::Xfs => "xfs",
            FileSystem::Swap => "swap",
            FileSystem::Fat32 => "fat32",
            FileSystem::Other(value) => value.as_str(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PartitionFlag {
    Boot,
    Esp,
    Swap,
    Lvm,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccount {
    pub username: String,
    pub full_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_hash: Option<String>,
    pub is_admin: bool,
}

impl UserAccount {
    pub fn default_admin() -> Self {
        Self {
            username: "armdistro".into(),
            full_name: "Arm Distro".into(),
            password_hash: None,
            is_admin: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub hostname: String,
    pub enable_network_manager: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            hostname: "arm-distro".into(),
            enable_network_manager: true,
        }
    }
}
