// Meeting detection module
// Detect ACTIVE meetings (not just running apps) using window title analysis
// Scans ALL open windows, not just the active one

use std::collections::HashSet;
use x_win::{get_open_windows, WindowInfo};

/// Pattern for detecting an active meeting based on window titles
#[derive(Debug, Clone)]
pub struct MeetingPattern {
    /// App name hint (optional, for faster filtering)
    pub app_hint: &'static str,
    /// Window title patterns - if any matches, it's an active meeting
    pub title_patterns: &'static [&'static str],
    /// Display name for notifications
    pub display_name: &'static str,
}

/// Known meeting patterns - window titles that indicate ACTIVE meetings
const MEETING_PATTERNS: &[MeetingPattern] = &[
    // Microsoft Teams: Active call shows "| Microsoft Teams" with meeting info
    // e.g., "Meeting Name | Microsoft | user@email.com | Microsoft Teams"
    MeetingPattern {
        app_hint: "teams",
        title_patterns: &[
            "| Microsoft Teams",
            "| Microsoft |",
            "Meeting with",
            "Teams Meeting",
        ],
        display_name: "Microsoft Teams",
    },
    // Zoom: Window title contains "Zoom Meeting" when in an active call
    MeetingPattern {
        app_hint: "zoom",
        title_patterns: &[
            "Zoom Meeting",
            "zoom meeting",
        ],
        display_name: "Zoom",
    },
    // Google Meet: Browser tab with meet.google.com or specific patterns
    MeetingPattern {
        app_hint: "", // Any browser
        title_patterns: &[
            "meet.google.com",
            "Meet -",
            "Google Meet",
        ],
        display_name: "Google Meet",
    },
    // Webex: Active meeting window
    MeetingPattern {
        app_hint: "webex",
        title_patterns: &[
            "Webex Meeting",
            "Meeting |",
            "Cisco Webex",
        ],
        display_name: "Webex",
    },
    // Slack Huddle: When in a huddle
    MeetingPattern {
        app_hint: "slack",
        title_patterns: &[
            "Huddle",
            "huddle",
        ],
        display_name: "Slack Huddle",
    },
    // Discord: Voice channel active
    MeetingPattern {
        app_hint: "discord",
        title_patterns: &[
            "Voice Connected",
            "Stage Channel",
        ],
        display_name: "Discord",
    },
];

/// Result of meeting detection
#[derive(Debug, Clone, PartialEq)]
pub struct DetectedMeeting {
    pub app_name: String,
    pub process_name: String,
}

pub struct MeetingDetector {
    #[allow(dead_code)]
    last_detected: Option<DetectedMeeting>,
    /// Track which apps we've already notified about (to avoid spam)
    notified_apps: HashSet<String>,
}

impl MeetingDetector {
    pub fn new() -> Self {
        Self {
            last_detected: None,
            notified_apps: HashSet::new(),
        }
    }

    /// Scan ALL open windows for meeting indicators
    /// Returns Some(DetectedMeeting) if a new meeting is detected (not previously notified)
    pub fn detect_meeting(&mut self) -> Option<DetectedMeeting> {
        // Get ALL open windows
        let windows = match get_open_windows() {
            Ok(w) => w,
            Err(e) => {
                log::debug!("Failed to get open windows: {:?}", e);
                return None;
            }
        };

        log::debug!("Scanning {} open windows for meetings", windows.len());

        // Check each window against meeting patterns
        for window in &windows {
            if let Some(detected) = self.check_window_for_meeting(window) {
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

        // No active meeting found - clear last detected and notified apps
        if self.last_detected.is_some() {
            log::debug!("No active meeting found in any window");
            self.last_detected = None;
            self.notified_apps.clear();
        }

        None
    }

    /// Check a single window against all meeting patterns
    fn check_window_for_meeting(&self, window: &WindowInfo) -> Option<DetectedMeeting> {
        let title = window.title.to_lowercase();
        let app_name = window.info.name.to_lowercase();
        let exec_name = window.info.exec_name.to_lowercase();

        // Skip windows with empty titles (no Screen Recording permission or minimized)
        if title.is_empty() {
            return None;
        }

        log::trace!("Checking window - app: '{}', exec: '{}', title: '{}'",
            app_name, exec_name, window.title);

        for pattern in MEETING_PATTERNS {
            // Check if app name or exec name matches (if specified)
            let app_matches = pattern.app_hint.is_empty()
                || app_name.contains(pattern.app_hint)
                || exec_name.contains(pattern.app_hint);

            if !app_matches {
                continue;
            }

            // Check if any title pattern matches
            let title_matches = pattern.title_patterns
                .iter()
                .any(|p| title.contains(&p.to_lowercase()));

            if title_matches {
                log::info!("Active meeting detected: {} (window: '{}', app: '{}')",
                    pattern.display_name, window.title, window.info.name);
                return Some(DetectedMeeting {
                    app_name: pattern.display_name.to_string(),
                    process_name: window.info.name.clone(),
                });
            }
        }

        None
    }

    /// Check if any meeting app is currently running (without notification tracking)
    pub fn is_meeting_running(&mut self) -> Option<DetectedMeeting> {
        let windows = match get_open_windows() {
            Ok(w) => w,
            Err(_) => return None,
        };

        for window in &windows {
            if let Some(detected) = self.check_window_for_meeting(window) {
                return Some(detected);
            }
        }

        None
    }

    /// Get the last detected meeting (if any)
    #[allow(dead_code)]
    pub fn get_last_detected(&self) -> Option<&DetectedMeeting> {
        self.last_detected.as_ref()
    }

    /// Clear notification tracking (call when user dismisses notification or starts recording)
    pub fn clear_notifications(&mut self) {
        self.notified_apps.clear();
    }

    /// Check if a specific app has been notified
    #[allow(dead_code)]
    pub fn was_notified(&self, app_name: &str) -> bool {
        self.notified_apps.contains(app_name)
    }

    /// Refresh is a no-op now (we get fresh window list each time)
    pub fn refresh(&mut self) {
        // No-op - window detection doesn't need refresh
    }

    /// Check if we have Screen Recording permission by testing if we can read other apps' window titles
    /// Returns true if at least one non-PhantomEar window has a non-empty title
    pub fn has_screen_recording_permission() -> bool {
        let windows = match get_open_windows() {
            Ok(w) => w,
            Err(e) => {
                log::warn!("Failed to get windows for permission check: {:?}", e);
                return false;
            }
        };

        // Count windows with non-empty titles that aren't PhantomEar
        let mut other_app_with_title = false;
        let mut other_app_without_title = false;

        for window in &windows {
            let app_name = window.info.name.to_lowercase();
            let exec_name = window.info.exec_name.to_lowercase();

            // Skip our own app
            if app_name.contains("phantom") || exec_name.contains("phantom") {
                continue;
            }

            // Skip system processes that might not have titles
            if app_name.is_empty() && exec_name.is_empty() {
                continue;
            }

            if !window.title.is_empty() {
                other_app_with_title = true;
                log::debug!("Found other app with title: '{}' (app: '{}')", window.title, window.info.name);
            } else {
                other_app_without_title = true;
                log::debug!("Found other app WITHOUT title: app='{}', exec='{}'", window.info.name, window.info.exec_name);
            }
        }

        // If we found at least one other app with a title, we likely have permission
        // If we only found apps without titles, permission is likely denied
        if other_app_with_title {
            log::info!("Screen Recording permission check: GRANTED (found apps with readable titles)");
            true
        } else if other_app_without_title {
            log::info!("Screen Recording permission check: DENIED (found apps but titles are empty)");
            false
        } else {
            // No other apps running, can't determine
            log::info!("Screen Recording permission check: UNKNOWN (no other apps found)");
            true // Assume granted if we can't tell
        }
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
    fn test_meeting_patterns_defined() {
        assert!(!MEETING_PATTERNS.is_empty());
        // Check that Teams is in the list
        assert!(MEETING_PATTERNS.iter().any(|p| p.display_name == "Microsoft Teams"));
        // Check that Zoom is in the list
        assert!(MEETING_PATTERNS.iter().any(|p| p.display_name == "Zoom"));
    }

    #[test]
    fn test_notification_tracking() {
        let mut detector = MeetingDetector::new();
        detector.notified_apps.insert("Zoom".to_string());
        assert!(detector.was_notified("Zoom"));
        assert!(!detector.was_notified("Teams"));
        detector.clear_notifications();
        assert!(!detector.was_notified("Zoom"));
    }
}
