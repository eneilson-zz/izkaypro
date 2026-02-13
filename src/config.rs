use serde::Deserialize;
use std::fs;
use std::path::Path;

use crate::kaypro_machine::VideoMode;
use crate::media::MediaFormat;

/// Configuration file name
const CONFIG_FILE: &str = "izkaypro.toml";

/// Kaypro model presets
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub enum KayproModel {
    /// Kaypro II - SSDD, memory-mapped video
    #[serde(rename = "kaypro_ii")]
    KayproII,
    /// Kaypro 4/83 - DSDD, memory-mapped video  
    #[serde(rename = "kaypro4_83")]
    Kaypro4_83,
    /// Kaypro 2X/4/84 - DSDD, SY6545 CRTC
    #[serde(rename = "kaypro4_84")]
    Kaypro4_84,
    /// Kaypro 4/84 with TurboROM 3.4 - DSDD, SY6545 CRTC
    #[serde(rename = "turbo_rom")]
    TurboRom,
    /// Kaypro 4/84 with KayPLUS replacement BIOS - DSDD, SY6545 CRTC
    #[serde(rename = "kayplus_84")]
    KayPlus84,
    /// Custom configuration (use rom_file, disk_format, video_mode)
    #[serde(rename = "custom")]
    Custom,
}

impl Default for KayproModel {
    fn default() -> Self {
        KayproModel::Kaypro4_84
    }
}

/// Video mode configuration
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VideoModeConfig {
    /// Memory-mapped VRAM at 0x3000-0x3FFF (Kaypro II, 4/83)
    MemoryMapped,
    /// SY6545 CRTC with port-based VRAM (Kaypro 2X/4/84)
    Sy6545,
}

impl Default for VideoModeConfig {
    fn default() -> Self {
        VideoModeConfig::Sy6545
    }
}

impl From<VideoModeConfig> for VideoMode {
    fn from(config: VideoModeConfig) -> Self {
        match config {
            VideoModeConfig::MemoryMapped => VideoMode::MemoryMapped,
            VideoModeConfig::Sy6545 => VideoMode::Sy6545Crtc,
        }
    }
}

/// Disk format configuration
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DiskFormatConfig {
    /// Single-sided double-density (200KB)
    Ssdd,
    /// Double-sided double-density (400KB)
    Dsdd,
}

impl Default for DiskFormatConfig {
    fn default() -> Self {
        DiskFormatConfig::Dsdd
    }
}

impl From<DiskFormatConfig> for MediaFormat {
    fn from(config: DiskFormatConfig) -> Self {
        match config {
            DiskFormatConfig::Ssdd => MediaFormat::SsDd,
            DiskFormatConfig::Dsdd => MediaFormat::DsDd,
        }
    }
}

/// Main configuration structure
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Kaypro model preset (overrides individual settings if not Custom)
    pub model: KayproModel,
    
    /// ROM file path (only used if model is Custom)
    pub rom_file: Option<String>,
    
    /// Video mode (only used if model is Custom)
    pub video_mode: VideoModeConfig,
    
    /// Disk format (only used if model is Custom)
    pub disk_format: DiskFormatConfig,
    
    /// Sector ID base for side 1 (only used if model is Custom)
    /// 10 = standard Kaypro (sectors 10-19 on side 1)
    /// 0 = KayPLUS format (sectors 0-9 on both sides)
    pub side1_sector_base: Option<u8>,
    
    /// Disk image for drive A (optional, overrides model default)
    pub disk_a: Option<String>,
    
    /// Disk image for drive B (optional, overrides model default)
    pub disk_b: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            model: KayproModel::default(),
            rom_file: None,
            video_mode: VideoModeConfig::default(),
            disk_format: DiskFormatConfig::default(),
            side1_sector_base: None,
            disk_a: None,
            disk_b: None,
        }
    }
}

impl Config {
    /// Load configuration from file, or return default if file doesn't exist
    pub fn load() -> Self {
        Self::load_from_path(CONFIG_FILE)
    }
    
    /// Load configuration from a specific path
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        if !path.exists() {
            return Config::default();
        }
        
        match fs::read_to_string(path) {
            Ok(content) => {
                match toml::from_str(&content) {
                    Ok(config) => config,
                    Err(e) => {
                        eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                        eprintln!("Using default configuration.");
                        Config::default()
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to read {}: {}", path.display(), e);
                eprintln!("Using default configuration.");
                Config::default()
            }
        }
    }

    /// Apply command-line overrides to the configuration.
    /// CLI arguments take priority over TOML config file settings.
    pub fn apply_cli_overrides(&mut self, model: Option<&str>, rom: Option<&str>, drivea: Option<&str>, driveb: Option<&str>) {
        if let Some(m) = model {
            self.model = match m {
                "kaypro_ii" => KayproModel::KayproII,
                "kaypro4_83" => KayproModel::Kaypro4_83,
                "kaypro4_84" => KayproModel::Kaypro4_84,
                "turbo_rom" => KayproModel::TurboRom,
                "kayplus_84" => KayproModel::KayPlus84,
                "custom" => KayproModel::Custom,
                _ => {
                    eprintln!("Warning: Unknown model '{}', using default", m);
                    self.model
                }
            };
        }
        if let Some(r) = rom {
            self.rom_file = Some(r.to_string());
            if self.model != KayproModel::Custom {
                self.model = KayproModel::Custom;
            }
        }
        if let Some(a) = drivea {
            self.disk_a = Some(a.to_string());
        }
        if let Some(b) = driveb {
            self.disk_b = Some(b.to_string());
        }
    }
    
    /// Get the ROM file path for this configuration
    pub fn get_rom_path(&self) -> &str {
        match self.model {
            KayproModel::KayproII => "roms/81-149c.rom",
            KayproModel::Kaypro4_83 => "roms/81-232.rom",
            KayproModel::Kaypro4_84 => "roms/81-292a.rom",
            KayproModel::TurboRom => "roms/trom34.rom",
            KayproModel::KayPlus84 => "roms/kplus84.rom",
            KayproModel::Custom => self.rom_file.as_deref().unwrap_or("roms/81-292a.rom"),
        }
    }
    
    /// Get the video mode for this configuration
    pub fn get_video_mode(&self) -> VideoMode {
        match self.model {
            KayproModel::KayproII => VideoMode::MemoryMapped,
            KayproModel::Kaypro4_83 => VideoMode::MemoryMapped,
            KayproModel::Kaypro4_84 => VideoMode::Sy6545Crtc,
            KayproModel::TurboRom => VideoMode::Sy6545Crtc,
            KayproModel::KayPlus84 => VideoMode::Sy6545Crtc,
            KayproModel::Custom => self.video_mode.into(),
        }
    }
    
    /// Get the disk format for this configuration
    pub fn get_disk_format(&self) -> MediaFormat {
        match self.model {
            KayproModel::KayproII => MediaFormat::SsDd,
            KayproModel::Kaypro4_83 => MediaFormat::DsDd,
            KayproModel::Kaypro4_84 => MediaFormat::DsDd,
            KayproModel::TurboRom => MediaFormat::DsDd,
            KayproModel::KayPlus84 => MediaFormat::DsDd,
            KayproModel::Custom => self.disk_format.into(),
        }
    }
    
    /// Get the side 1 sector ID base for this configuration.
    /// Standard Kaypro disks use 10 (sectors 10-19 on side 1).
    /// KayPLUS-formatted disks use 0 (sectors 0-9 on both sides).
    pub fn get_side1_sector_base(&self) -> u8 {
        match self.model {
            KayproModel::KayPlus84 => 0,
            KayproModel::Custom => self.side1_sector_base.unwrap_or(10),
            _ => 10,
        }
    }
    
    /// Get the default boot disk path for this configuration
    pub fn get_default_disk_a(&self) -> &str {
        match self.model {
            KayproModel::KayproII => "disks/system/cpm22-rom149.img",
            KayproModel::Kaypro4_83 => "disks/system/k484-cpm22f-boot.img",
            KayproModel::Kaypro4_84 => "disks/system/cpm22g-rom292a.img",
            KayproModel::TurboRom => "disks/system/k484_turborom_63k_boot.img",
            KayproModel::KayPlus84 => "disks/system/kayplus_boot.img",
            KayproModel::Custom => self.disk_a.as_deref().unwrap_or("disks/system/k484-cpm22f-boot.img"),
        }
    }
    
    /// Get the default disk B path for this configuration
    pub fn get_default_disk_b(&self) -> &str {
        match self.model {
            KayproModel::KayproII => "disks/blank_disks/cpm22-rom149-blank.img",
            KayproModel::Kaypro4_83 => "disks/blank_disks/cpm22-kaypro4-blank.img",
            KayproModel::Kaypro4_84 => "disks/blank_disks/cpm22-kaypro4-blank.img",
            KayproModel::TurboRom => "disks/blank_disks/cpm22-kaypro4-blank.img",
            KayproModel::KayPlus84 => "disks/blank_disks/cpm22-kaypro4-blank.img",
            KayproModel::Custom => self.disk_b.as_deref().unwrap_or("disks/blank_disks/cpm22-kaypro4-blank.img"),
        }
    }
    
    /// Get a description of this configuration
    pub fn get_description(&self) -> String {
        match self.model {
            KayproModel::KayproII => "Kaypro II (SSDD, 81-149c ROM)".to_string(),
            KayproModel::Kaypro4_83 => "Kaypro 4/83 (DSDD, 81-232 ROM)".to_string(),
            KayproModel::Kaypro4_84 => "Kaypro 2X/4/84 (DSDD, 81-292a ROM)".to_string(),
            KayproModel::TurboRom => "Kaypro 4/84 TurboROM 3.4 (DSDD)".to_string(),
            KayproModel::KayPlus84 => "Kaypro 4/84 KayPLUS (DSDD)".to_string(),
            KayproModel::Custom => format!("Custom ({})", self.get_rom_path()),
        }
    }
    
    /// Get a short user-friendly name for display in the emulator title
    pub fn get_display_name(&self) -> &str {
        match self.model {
            KayproModel::KayproII => "Kaypro II",
            KayproModel::Kaypro4_83 => "Kaypro 4/83",
            KayproModel::Kaypro4_84 => "Kaypro 4-84",
            KayproModel::TurboRom => "Kaypro 4-84 TurboROM",
            KayproModel::KayPlus84 => "Kaypro 4-84 KayPLUS",
            KayproModel::Custom => "Custom Kaypro",
        }
    }
}
