use gyg_eventsource::{Dto, State};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use template_shared::command::TemplateCommand;
use template_shared::error::TemplateError;
use template_shared::event::{Delayed, TemplateEvent};
use template_shared::START_VALUE;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct TemplateState {
    value: usize,
    last_id: usize,
    delayed: Vec<Delayed>,
}

impl TemplateState {
    pub fn get_value(&self) -> usize {
        self.value
    }
}

impl Default for TemplateState {
    fn default() -> Self {
        TemplateState {
            value: START_VALUE,
            last_id: 0,
            delayed: vec![],
        }
    }
}

impl Dto for TemplateState {
    type Event = TemplateEvent;
    type Error = TemplateError;

    fn play_event(&mut self, event: &Self::Event) {
        match event {
            TemplateEvent::Added(i) => self.value += i,
            TemplateEvent::Removed(i) => self.value -= i,
            TemplateEvent::Delayed(d) => {
                self.last_id = d.id;
                self.delayed.push(d.clone());
            },
            TemplateEvent::DelayDone(id) => {
                self.delayed = self.delayed.clone().into_iter().filter(|d| d.id != *id).collect();
            }
        }
    }
}

impl State for TemplateState {
    type Command = TemplateCommand;

    fn try_command(&self, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            TemplateCommand::Add(i) => {
                if self.value + i > 3000 {
                    Err(Self::Error::CannotAdd(i))
                } else {
                    Ok(vec![Self::Event::Added(i)])
                }
            }
            TemplateCommand::Reset => {
                if self.value == 0 {
                    Err(Self::Error::AlreadyEmpty)
                } else {
                    Ok(vec![Self::Event::Removed(self.value)])
                }
            }
            TemplateCommand::Delayed(d) => {
                if d.delay < 1 || d.delay > 10 {
                    return Err(Self::Error::DelayOutOfBound(d.delay));
                }

                let now = SystemTime::now();
                let duration = Duration::new(d.delay as u64, 0);
                let end = now + duration;
                let end = end
                    .duration_since(UNIX_EPOCH)
                    .map_err(|_| Self::Error::CannotCalculateTime)?;

                Ok(vec![Self::Event::Delayed(Delayed {
                    id : self.last_id +1,
                    timestamp: end.as_secs(),
                    to_add: d.to_add,
                })])
            }
            TemplateCommand::Finalize(id) => {

                let now = SystemTime::now();
                let epoch = now.duration_since(UNIX_EPOCH)
                    .map_err(|_| Self::Error::CannotCalculateTime)?.as_secs();

                for i in 0..self.delayed.len(){
                    if self.delayed[i].id == id && self.delayed[i].timestamp >= epoch {
                        return Ok(vec![
                            Self::Event::DelayDone(id),
                            Self::Event::Added( self.delayed[i].to_add)
                        ])
                    }
                }

                Err(Self::Error::DelayNotFound)
            }
        }
    }
}
