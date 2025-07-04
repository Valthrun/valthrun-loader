use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Artifact {
    Cs2Overlay,
    Cs2RadarClient,
    DriverInterfaceKernel,
    KernelDriver,
}

impl Artifact {
    pub const fn name(&self) -> &'static str {
        match self {
            Artifact::Cs2Overlay => "CS2 Overlay",
            Artifact::Cs2RadarClient => "CS2 Radar Client",
            Artifact::DriverInterfaceKernel => "Driver Interface Kernel",
            Artifact::KernelDriver => "Kernel Driver",
        }
    }

    pub const fn slug(&self) -> &'static str {
        match self {
            Artifact::Cs2Overlay => "cs2-overlay",
            Artifact::Cs2RadarClient => "cs2-radar-client",
            Artifact::DriverInterfaceKernel => "driver-interface-kernel",
            Artifact::KernelDriver => "kernel-driver",
        }
    }

    pub const fn file_name(&self) -> &'static str {
        match self {
            Artifact::Cs2Overlay => "cs2_overlay.exe",
            Artifact::Cs2RadarClient => "cs2_radar_client.exe",
            Artifact::DriverInterfaceKernel => "driver_interface_kernel.dll",
            Artifact::KernelDriver => "kernel_driver.sys",
        }
    }
}

#[derive(ValueEnum, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[clap(rename_all = "kebab-case")]
pub enum Enhancer {
    Cs2Overlay,
    Cs2StandaloneRadar,
}

impl Enhancer {
    pub const fn required_artifacts(&self) -> &'static [&'static Artifact] {
        match self {
            Enhancer::Cs2Overlay => &[&Artifact::Cs2Overlay, &Artifact::DriverInterfaceKernel],
            Enhancer::Cs2StandaloneRadar => {
                &[&Artifact::Cs2RadarClient, &Artifact::DriverInterfaceKernel]
            }
        }
    }

    pub const fn artifact_to_execute(&self) -> &'static Artifact {
        match self {
            Enhancer::Cs2Overlay => &Artifact::Cs2Overlay,
            Enhancer::Cs2StandaloneRadar => &Artifact::Cs2RadarClient,
        }
    }
}
