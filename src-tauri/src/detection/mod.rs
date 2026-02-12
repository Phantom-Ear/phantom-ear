// Meeting detection module
// Detect running meeting applications (Zoom, Teams, Google Meet, etc.)

use sysinfo::System;
use std::collections::HashSet;

/// Meeting app detection info
#[derive(Debug, Clone)]
pub struct MeetingApp {
    /// Process name pattern (case-insensitive partial match)
    pub process_pattern: &'static str,
    /// Display name for notifications
    pub display_name: &'static str,
}

/// Known meeting application process names
const MEETING_APPS: &[MeetingApp] = &[
    // Zoom
    MeetingApp { process_pattern: "zoom", display_name: "Zoom" },
    // Microsoft Teams
    MeetingApp { process_pattern: "teams", display_name: "Microsoft Teams" },
    // Slack Huddle
    MeetingApp { process_pattern: "slack", display_name: "Slack" },
    // Discord
    MeetingApp { process_pattern: "discord", display_name: "Discord" },
    // Webex
    MeetingApp { process_pattern: "webex", display_name: "Webex" },
    // Google Meet runs in browser - detected by specific process patterns
    MeetingApp { process_pattern: "meet.google", display_name: "Google Meet" },
];

/// Result of meeting detection
#[derive(Debug, Clone, PartialEq)]
pub struct DetectedMeeting {
    pub app_name: String,
    pub process_name: String,
}

pub struct MeetingDetector {
    system: System,
    last_detected: Option<DetectedMeeting>,
    /// Track which apps we've already notified about (to avoid spam)
    notified_apps: HashSet<String>,
}

impl MeetingDetector {
    pub fn new() -> Self {
        Self {
            system: System::new(),
            last_detected: None,
            notified_apps: HashSet::new(),
        }
    }

    /// Refresh system process list
    pub fn refresh(&mut self) {
        self.system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    }

    /// Check if a meeting app is currently running
    /// Returns Some(DetectedMeeting) if a new meeting is detected (not previously notified)
    pub fn detect_meeting(&mut self) -> Option<DetectedMeeting> {
        self.refresh();

        // Get all running process names
        let processes: Vec<String> = self.system
            .processes()
            .values()
            .map(|p| p.name().to_string_lossy().to_lowercase())
            .collect();

        // Check each known meeting app
        for app in MEETING_APPS {
            let pattern = app.process_pattern.to_lowercase();

            for proc_name in &processes {
                if proc_name.contains(&pattern) {
                    let detected = DetectedMeeting {
                        app_name: app.display_name.to_string(),
                        process_name: proc_name.clone(),
                    };

                    // Check if we've already notified about this app
                    if !self.notified_apps.contains(&detected.app_name) {
                        self.notified_apps.insert(detected.app_name.clone());
                        self.last_detected = Some(detected.clone());
                        return Some(detected);
                    } else {
                        // Already notified, just update last_detected
                        self.last_detected = Some(detected);
                        return None;
                    }
                }
            }
        }

        // No meeting app found - clear last detected and notified apps
        if self.last_detected.is_some() {
            self.last_detected = None;
            self.notified_apps.clear();
        }

        None
    }

    /// Check if any meeting app is currently running (without notification tracking)
    pub fn is_meeting_running(&mut self) -> Option<DetectedMeeting> {
        self.refresh();

        let processes: Vec<String> = self.system
            .processes()
            .values()
            .map(|p| p.name().to_string_lossy().to_lowercase())
            .collect();

        for app in MEETING_APPS {
            let pattern = app.process_pattern.to_lowercase();

            for proc_name in &processes {
                if proc_name.contains(&pattern) {
                    return Some(DetectedMeeting {
                        app_name: app.display_name.to_string(),
                        process_name: proc_name.clone(),
                    });
                }
            }
        }

        None
    }

    /// Get the last detected meeting (if any)
    pub fn get_last_detected(&self) -> Option<&DetectedMeeting> {
        self.last_detected.as_ref()
    }

    /// Clear notification tracking (call when user dismisses notification or starts recording)
    pub fn clear_notifications(&mut self) {
        self.notified_apps.clear();
    }

    /// Check if a specific app has been notified
    pub fn was_notified(&self, app_name: &str) -> bool {
        self.notified_apps.contains(app_name)
    }
}

impl Default for MeetingDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_creation() {
        let detector = MeetingDetector::new();
        assert!(detector.last_detected.is_none());
    }

    #[test]
    fn test_meeting_apps_defined() {
        assert!(!MEETING_APPS.is_empty());
        // Check that Zoom is in the list
        assert!(MEETING_APPS.iter().any(|app| app.display_name == "Zoom"));
    }
}
