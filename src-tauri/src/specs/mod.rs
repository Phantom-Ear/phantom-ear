// Device specifications detection module
// Detects CPU, RAM, GPU capabilities for model recommendations

use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSpecs {
    /// Number of physical CPU cores
    pub cpu_cores: usize,
    /// Number of logical CPU threads
    pub cpu_threads: usize,
    /// CPU brand/model name
    pub cpu_name: String,
    /// Total RAM in gigabytes
    pub ram_gb: f64,
    /// Available RAM in gigabytes
    pub available_ram_gb: f64,
    /// Whether the device has a GPU
    pub has_gpu: bool,
    /// GPU name (if detected)
    pub gpu_name: Option<String>,
    /// Whether running on Apple Silicon
    pub is_apple_silicon: bool,
    /// Operating system
    pub os: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecommendation {
    /// Recommended model name
    pub recommended_model: String,
    /// Reason for the recommendation
    pub reason: String,
    /// Estimated relative speed (1-5, 5 being fastest)
    pub estimated_speed: u8,
    /// All models with compatibility info
    pub model_compatibility: Vec<ModelCompatibility>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCompatibility {
    pub name: String,
    pub compatible: bool,
    pub reason: String,
}

impl DeviceSpecs {
    /// Detect device specifications
    pub fn detect() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        // CPU info
        let cpu_cores = num_cpus::get_physical();
        let cpu_threads = num_cpus::get();
        let cpu_name = sys
            .cpus()
            .first()
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string());

        // RAM info (in bytes, convert to GB)
        let total_memory = sys.total_memory();
        let available_memory = sys.available_memory();
        let ram_gb = total_memory as f64 / (1024.0 * 1024.0 * 1024.0);
        let available_ram_gb = available_memory as f64 / (1024.0 * 1024.0 * 1024.0);

        // Check for Apple Silicon
        let is_apple_silicon = Self::detect_apple_silicon(&cpu_name);

        // GPU detection
        let (has_gpu, gpu_name) = Self::detect_gpu(is_apple_silicon);

        // OS info
        let os = System::name().unwrap_or_else(|| "Unknown".to_string());

        DeviceSpecs {
            cpu_cores,
            cpu_threads,
            cpu_name,
            ram_gb,
            available_ram_gb,
            has_gpu,
            gpu_name,
            is_apple_silicon,
            os,
        }
    }

    fn detect_apple_silicon(cpu_name: &str) -> bool {
        let name_lower = cpu_name.to_lowercase();
        name_lower.contains("apple m")
            || name_lower.contains("apple m1")
            || name_lower.contains("apple m2")
            || name_lower.contains("apple m3")
            || name_lower.contains("apple m4")
    }

    fn detect_gpu(is_apple_silicon: bool) -> (bool, Option<String>) {
        if is_apple_silicon {
            // Apple Silicon has integrated GPU
            return (true, Some("Apple Silicon GPU".to_string()));
        }

        // For other platforms, we'd need platform-specific detection
        // For now, assume no dedicated GPU on non-Apple Silicon
        // This could be expanded with wgpu or platform-specific APIs
        #[cfg(target_os = "macos")]
        {
            // On macOS, even Intel Macs have integrated GPUs
            (true, Some("Integrated GPU".to_string()))
        }

        #[cfg(not(target_os = "macos"))]
        {
            // On other platforms, we can't easily detect without additional dependencies
            (false, None)
        }
    }
}

impl ModelRecommendation {
    /// Generate model recommendation based on device specs
    pub fn from_specs(specs: &DeviceSpecs) -> Self {
        let mut compatibility = Vec::new();

        // Define model requirements
        let models = [
            ("tiny", 75, 2.0, 2), // (name, size_mb, min_ram_gb, min_cores)
            ("base", 142, 4.0, 2),
            ("small", 466, 6.0, 4),
            ("medium", 1500, 10.0, 6),
            ("large", 2900, 16.0, 8),
        ];

        let mut recommended = "tiny".to_string();
        let mut reason = "Default fallback for limited resources".to_string();
        let mut speed = 5u8;

        for (name, _size_mb, min_ram, min_cores) in models.iter() {
            let has_enough_ram = specs.ram_gb >= *min_ram;
            let has_enough_cores = specs.cpu_cores >= *min_cores;
            let compatible = has_enough_ram && has_enough_cores;

            let compat_reason = if !has_enough_ram && !has_enough_cores {
                format!("Needs {}GB RAM and {} cores", min_ram, min_cores)
            } else if !has_enough_ram {
                format!("Needs {}GB RAM (you have {:.1}GB)", min_ram, specs.ram_gb)
            } else if !has_enough_cores {
                format!("Needs {} cores (you have {})", min_cores, specs.cpu_cores)
            } else {
                "Compatible with your system".to_string()
            };

            compatibility.push(ModelCompatibility {
                name: name.to_string(),
                compatible,
                reason: compat_reason,
            });

            // Update recommendation if this model is compatible
            if compatible {
                recommended = name.to_string();
            }
        }

        // Generate recommendation reason and speed
        match recommended.as_str() {
            "large" => {
                reason =
                    "Your system has excellent specs - large model will give the best accuracy"
                        .to_string();
                speed = 2;
            }
            "medium" => {
                reason =
                    "Your system can handle the medium model for great accuracy with good speed"
                        .to_string();
                speed = 3;
            }
            "small" => {
                reason = "Small model offers a good balance of accuracy and speed for your system"
                    .to_string();
                speed = 4;
            }
            "base" => {
                reason = "Base model recommended - good accuracy with fast real-time performance"
                    .to_string();
                speed = 5;
            }
            "tiny" => {
                reason = "Tiny model for fastest performance on your system".to_string();
                speed = 5;
            }
            _ => {}
        }

        // Boost recommendation for Apple Silicon
        if specs.is_apple_silicon && specs.has_gpu {
            reason = format!("{} Apple Silicon GPU acceleration available.", reason);
            speed = speed.saturating_add(1).min(5);
        }

        ModelRecommendation {
            recommended_model: recommended,
            reason,
            estimated_speed: speed,
            model_compatibility: compatibility,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_specs() {
        let specs = DeviceSpecs::detect();
        assert!(specs.cpu_cores > 0);
        assert!(specs.cpu_threads > 0);
        assert!(specs.ram_gb > 0.0);
    }

    #[test]
    fn test_model_recommendation() {
        let specs = DeviceSpecs {
            cpu_cores: 8,
            cpu_threads: 16,
            cpu_name: "Test CPU".to_string(),
            ram_gb: 16.0,
            available_ram_gb: 8.0,
            has_gpu: true,
            gpu_name: Some("Test GPU".to_string()),
            is_apple_silicon: false,
            os: "Test OS".to_string(),
        };
        let rec = ModelRecommendation::from_specs(&specs);
        assert_eq!(rec.recommended_model, "medium");
    }
}
