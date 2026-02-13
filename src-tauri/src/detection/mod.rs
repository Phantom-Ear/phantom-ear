// Meeting detection module
// Detect ACTIVE meetings (not just running apps) using window title analysis

use active_win_pos_rs::get_active_window;
use std::collections::HashSet;

/// Pattern for detecting an active meeting based on window titles
#[derive(Debug, Clone)]
pub struct MeetingPattern {
    /// Process name hint (optional, for faster filtering)
    pub process_hint: &'static str,
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
        process_hint: "teams",
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
        process_hint: "zoom",
        title_patterns: &[
            "Zoom Meeting",
            "zoom meeting",
        ],
        display_name: "Zoom",
    },
    // Google Meet: Browser tab with meet.google.com or specific patterns
    MeetingPattern {
        process_hint: "", // Any browser
        title_patterns: &[
            "meet.google.com",
            "Meet -",
            "Google Meet",
        ],
        display_name: "Google Meet",
    },
    // Webex: Active meeting window
    MeetingPattern {
        process_hint: "webex",
        title_patterns: &[
            "Webex Meeting",
            "Meeting |",
            "Cisco Webex",
        ],
        display_name: "Webex",
    },
    // Slack Huddle: When in a huddle
    MeetingPattern {
        process_hint: "slack",
        title_patterns: &[
            "Huddle",
            "huddle",
        ],
        display_name: "Slack Huddle",
    },
    // Discord: Voice channel active
    MeetingPattern {
        process_hint: "discord",
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
    /// Cache of window titles we've seen (for debugging)
    #[allow(dead_code)]
    last_window_title: Option<String>,
}

impl MeetingDetector {
    pub fn new() -> Self {
        Self {
            last_detected: None,
            notified_apps: HashSet::new(),
            last_window_title: None,
        }
    }

    /// Check if an active meeting is detected by analyzing the active window
    /// Returns Some(DetectedMeeting) if a new meeting is detected (not previously notified)
    pub fn detect_meeting(&mut self) -> Option<DetectedMeeting> {
        // Get the currently active window
        let active_window = match get_active_window() {
            Ok(window) => window,
            Err(e) => {
                log::debug!("Failed to get active window: {:?}", e);
                return None;
            }
        };

        let title = active_window.title.to_lowercase();
        let app_name = active_window.app_name.to_lowercase();

        self.last_window_title = Some(active_window.title.clone());

        log::debug!("Active window - app: {}, title: {}", app_name, active_window.title);

        // Check each meeting pattern
        for pattern in MEETING_PATTERNS {
            // Check if app name matches (if specified)
            let app_matches = pattern.process_hint.is_empty()
                || app_name.contains(pattern.process_hint);

            if !app_matches {
                continue;
            }

            // Check if any title pattern matches
            let title_matches = pattern.title_patterns
                .iter()
                .any(|p| title.contains(&p.to_lowercase()));

            if title_matches {
                let detected = DetectedMeeting {
                    app_name: pattern.display_name.to_string(),
                    process_name: active_window.app_name.clone(),
                };

                // Check if we've already notified about this app
                if !self.notified_apps.contains(&detected.app_name) {
                    self.notified_apps.insert(detected.app_name.clone());
                    self.last_detected = Some(detected.clone());
                    log::info!("Active meeting detected: {} (window: {})",
                        pattern.display_name, active_window.title);
                    return Some(detected);
                } else {
                    // Already notified, just update last_detected
                    self.last_detected = Some(detected);
                    return None;
                }
            }
        }

        // No active meeting found - clear last detected and notified apps
        // This allows re-notification if user joins another meeting later
        if self.last_detected.is_some() {
            log::debug!("No active meeting in current window");
            self.last_detected = None;
            self.notified_apps.clear();
        }

        None
    }

    /// Check if any meeting app is currently running (without notification tracking)
    /// This is a simpler check that just returns if an active meeting is found
    pub fn is_meeting_running(&mut self) -> Option<DetectedMeeting> {
        let active_window = match get_active_window() {
            Ok(window) => window,
            Err(_) => return None,
        };

        let title = active_window.title.to_lowercase();
        let app_name = active_window.app_name.to_lowercase();

        for pattern in MEETING_PATTERNS {
            let app_matches = pattern.process_hint.is_empty()
                || app_name.contains(pattern.process_hint);

            if !app_matches {
                continue;
            }

            let title_matches = pattern.title_patterns
                .iter()
                .any(|p| title.contains(&p.to_lowercase()));

            if title_matches {
                return Some(DetectedMeeting {
                    app_name: pattern.display_name.to_string(),
                    process_name: active_window.app_name.clone(),
                });
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

    /// Refresh is a no-op now (we don't need sysinfo refresh for window detection)
    pub fn refresh(&mut self) {
        // No-op - window detection doesn't need refresh
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
