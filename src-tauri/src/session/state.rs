// 会话状态机（纯逻辑，不依赖 IPC/IO）
use crate::domain::session::SessionStatus;
use crate::error::{AppError, ErrorScope};

// 状态机事件
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEvent {
    Start,
    End,
    Segment,
    SummarizeOk,
    Fail,
    Reset,
}

// 会话状态机
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionStateMachine {
    status: SessionStatus,
    segment_count: u32,
}

impl Default for SessionStateMachine {
    fn default() -> Self {
        Self {
            status: SessionStatus::Idle,
            segment_count: 0,
        }
    }
}

impl SessionStateMachine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn status(&self) -> SessionStatus {
        self.status
    }

    pub fn segment_count(&self) -> u32 {
        self.segment_count
    }

    // 处理事件并返回新状态
    pub fn handle(&mut self, event: SessionEvent) -> Result<SessionStatus, AppError> {
        use SessionEvent::*;
        use SessionStatus::*;

        match (self.status, event) {
            (Idle, Start) | (Completed, Start) | (Error, Start) => {
                self.status = Recording;
                self.segment_count = 0;
            }
            (Recording, End) => self.status = Summarizing,
            (Recording, Segment) => self.segment_count += 1,
            (Summarizing, SummarizeOk) => self.status = Completed,
            (Idle, Fail) | (Recording, Fail) | (Summarizing, Fail) => self.status = Error,
            (Completed, Reset) | (Error, Reset) => {
                self.status = Idle;
                self.segment_count = 0;
            }
            (Idle, Segment) | (Summarizing, Segment) => {}
            (status, event) => {
                return Err(AppError::new(
                    ErrorScope::Session,
                    format!("非法操作：{status:?} 状态下不允许 {event:?}"),
                ));
            }
        }
        Ok(self.status)
    }
}

#[cfg(test)]
mod tests {
    use super::SessionEvent::*;
    use super::SessionStatus::*;
    use super::*;

    #[test]
    fn idle_start_to_recording() {
        let mut m = SessionStateMachine::new();
        assert_eq!(m.status(), Idle);
        assert_eq!(m.handle(Start).unwrap(), Recording);
    }

    #[test]
    fn full_flow_to_completed_then_reset() {
        let mut m = SessionStateMachine::new();
        m.handle(Start).unwrap();
        m.handle(Segment).unwrap();
        m.handle(Segment).unwrap();
        assert_eq!(m.segment_count(), 2);
        assert_eq!(m.handle(End).unwrap(), Summarizing);
        assert_eq!(m.handle(SummarizeOk).unwrap(), Completed);
        assert_eq!(m.handle(Reset).unwrap(), Idle);
    }

    #[test]
    fn segment_counted_only_in_recording() {
        let mut m = SessionStateMachine::new();
        m.handle(Segment).unwrap();
        assert_eq!(m.segment_count(), 0);
        m.handle(Start).unwrap();
        m.handle(Segment).unwrap();
        assert_eq!(m.segment_count(), 1);
        m.handle(End).unwrap();
        m.handle(Segment).unwrap();
        assert_eq!(m.segment_count(), 1);
    }

    #[test]
    fn fail_enters_error_and_can_restart() {
        let mut m = SessionStateMachine::new();
        assert_eq!(m.handle(Fail).unwrap(), Error);
        assert_eq!(m.handle(Start).unwrap(), Recording);
        m.handle(Segment).unwrap();
        assert_eq!(m.handle(Fail).unwrap(), Error);
        assert_eq!(m.handle(Reset).unwrap(), Idle);
    }

    #[test]
    fn start_again_from_completed_clears_segments() {
        let mut m = SessionStateMachine::new();
        m.handle(Start).unwrap();
        m.handle(Segment).unwrap();
        m.handle(End).unwrap();
        m.handle(SummarizeOk).unwrap();
        assert_eq!(m.handle(Start).unwrap(), Recording);
        assert_eq!(m.segment_count(), 0);
    }

    #[test]
    fn illegal_operations_rejected() {
        let mut m = SessionStateMachine::new();
        assert!(m.handle(End).is_err());
        assert!(m.handle(SummarizeOk).is_err());
        assert!(m.handle(Reset).is_err());
        m.handle(Start).unwrap();
        assert!(m.handle(Start).is_err());
        assert!(m.handle(Reset).is_err());
        assert!(m.handle(SummarizeOk).is_err());
        m.handle(End).unwrap();
        assert!(m.handle(End).is_err());
        assert!(m.handle(Start).is_err());
        assert!(m.handle(Reset).is_err());
        m.handle(SummarizeOk).unwrap();
        assert!(m.handle(End).is_err());
        assert!(m.handle(SummarizeOk).is_err());
    }

    #[test]
    fn error_message_is_chinese() {
        let mut m = SessionStateMachine::new();
        let err = m.handle(End).unwrap_err();
        assert!(err.message.contains("非法操作"));
    }
}
