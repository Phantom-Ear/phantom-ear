// Meeting detection module
// Detect running meeting applications (Zoom, Teams, Google Meet)

/// Known meeting application process names
#[allow(dead_code)]
const MEETING_APPS: &[(&str, &str)] = &[
    ("zoom.us", "Zoom"),
    ("Zoom", "Zoom"),
    ("Microsoft Teams", "Microsoft Teams"),
    ("Teams", "Microsoft Teams"),
    ("Google Chrome", "Google Meet"), // Need additional window title check
    ("Arc", "Google Meet"),
    ("Safari", "Google Meet"),
    ("Firefox", "Google Meet"),
];

pub struct MeetingDetector {
    #[allow(dead_code)]
    last_detected: Option<String>,
}

impl MeetingDetector {
    pub fn new() -> Self {
        Self {
            last_detected: None,
        }
    }

    /// Check if a meeting app is currently running
    pub fn detect_meeting(&mut self) -> Option<String> {
        // TODO: Platform-specific process detection
        // macOS: Use sysctl or NSWorkspace
        // Windows: Use Windows API to enumerate processes

        #[cfg(target_os = "macos")]
        {
            self.detect_meeting_macos()
        }

        #[cfg(target_os = "windows")]
        {
            self.detect_meeting_windows()
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            None
        }
    }

    #[cfg(target_os = "macos")]
    fn detect_meeting_macos(&self) -> Option<String> {
        // TODO: Use NSWorkspace.runningApplications
        // Check against MEETING_APPS
        // For browsers, also check window title for "meet.google.com"
        None
    }

    #[cfg(target_os = "windows")]
    fn detect_meeting_windows(&self) -> Option<String> {
        // TODO: Use EnumProcesses / CreateToolhelp32Snapshot
        // Check against MEETING_APPS
        None
    }
}
