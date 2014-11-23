use std::comm;
use std::sync::RWLock;

#[cfg(test)]
mod test;

pub enum CommMsg<T, E> {
    Message(T),
    Error(E),
    Ping,
}

#[allow(dead_code)]
enum MaybeOwned<'a, A: 'a> {
    Owned(A),
    Borrowed(&'a A)
}

pub struct Sender<T, E> {
    closed: RWLock<bool>,
    inner: comm::Sender<CommMsg<T, E>>
}

pub struct Receiver<T, E> {
    closed: RWLock<bool>,
    error: RWLock<Option<E>>,
    inner: comm::Receiver<CommMsg<T, E>>
}

pub struct ReceiverIterator<'a, T: 'a, E: 'a> {
    reference: MaybeOwned<'a, Receiver<T, E>>,
    blocking: bool
}

impl <'a, A> MaybeOwned<'a A> {
    fn borrow<'b: 'a>(&'b self) -> &'b A  {
        match *self {
            MaybeOwned::Owned(ref a) => a,
            MaybeOwned::Borrowed(a) => a
        }
    }
}

pub fn channel<T, E>() -> (Sender<T, E>, Receiver<T, E>) where
T: Send, E: Send + Sync{
    let (tx, rx) = comm::channel();
    (Sender::from_old(tx), Receiver::from_old(rx))
}

impl <T, E> Sender<T, E> where T: Send, E: Send {
    pub fn from_old(v: comm::Sender<CommMsg<T, E>>) -> Sender<T, E> {
        Sender {
            closed: RWLock::new(false),
            inner: v
        }
    }

    pub fn into_inner(self) -> comm::Sender<CommMsg<T, E>> {
        self.inner
    }

    pub fn send(&self, t: T) -> Result<(), T> {
        match self.inner.send_opt(CommMsg::Message(t)) {
            Ok(()) => Ok(()),
            Err(CommMsg::Message(a)) => {
                * self.closed.write() = true;
                Err(a)
            },
            Err(_) => unreachable!()
        }
    }

    pub fn close(self) { }

    pub fn error(self, e: E) -> Result<(), E> {
        match self.inner.send_opt(CommMsg::Error(e)) {
            Ok(()) => Ok(()),
            Err(CommMsg::Error(a)) => {
                * self.closed.write() = true;
                Err(a)
            }
            Err(_) => unreachable!()
        }
    }

    pub fn is_closed(&self) -> bool {
        if * self.closed.read() {
            true
        } else {
            match self.inner.send_opt(CommMsg::Ping) {
                Ok(()) => false,
                Err(_) => {
                    * self.closed.write() = true;
                    true
                }
            }
        }
    }
}

impl <T, E> Clone for Sender<T, E> where T: Send, E: Send {
    fn clone(&self) -> Sender<T, E> {
        Sender {
            inner: self.inner.clone(),
            closed: RWLock::new(*self.closed.read())
        }
    }
}

impl <T, E> Receiver<T, E> where T: Send, E: Send + Sync {
    pub fn from_old(v: comm::Receiver<CommMsg<T, E>>) -> Receiver<T, E> {
        Receiver {
            closed: RWLock::new(false),
            error: RWLock::new(None),
            inner: v
        }
    }

    pub fn into_inner(self) -> (comm::Receiver<CommMsg<T, E>>, Option<E>) {
        (self.inner, self.error.write().take())
    }

    pub fn recv(&self) -> Option<T> {
        if self.is_closed() {
            return None
        }
        match self.inner.try_recv() {
            Ok(CommMsg::Message(m)) => Some(m),
            Ok(CommMsg::Error(e)) => {
                * self.error.write() = Some(e);
                * self.closed.write() = true;
                None
            }
            Ok(CommMsg::Ping) => {
                self.recv()
            }
            Err(comm::TryRecvError::Empty) => None,
            Err(comm::TryRecvError::Disconnected) => {
                * self.closed.write() = true;
                None
            }
        }
    }

    pub fn recv_block(&self) -> Option<T> {
        if self.is_closed() {
            return None
        }
        match self.inner.recv_opt() {
            Ok(CommMsg::Message(m)) => Some(m),
            Ok(CommMsg::Error(e)) => {
                * self.error.write() = Some(e);
                * self.closed.write() = true;
                None
            }
            Ok(CommMsg::Ping) => {
                self.recv_block()
            }
            Err(()) => {
                * self.closed.write() = true;
                None
            }
        }
    }

    pub fn has_error(&self) -> bool {
        self.error.read().is_some()
    }

    pub fn take_error(&self) -> Option<E> {
        self.error.write().take()
    }

    pub fn is_closed(&self) -> bool {
        * self.closed.read()
    }

    pub fn iter(&self) -> ReceiverIterator<T, E> {
        ReceiverIterator {
            blocking: false,
            reference: MaybeOwned::Borrowed(self)
        }
    }

    pub fn blocking_iter(&self) -> ReceiverIterator<T, E> {
        ReceiverIterator {
            blocking: true,
            reference: MaybeOwned::Borrowed(self)
        }
    }

    pub fn into_iter(self) -> ReceiverIterator<'static, T, E> {
        ReceiverIterator {
            blocking: false,
            reference: MaybeOwned::Owned(self)
        }
    }

    pub fn into_blocking_iter(self) -> ReceiverIterator<'static, T, E> {
        ReceiverIterator {
            blocking: true,
            reference: MaybeOwned::Owned(self)
        }
    }
}

impl <'a, T, E> Iterator<T> for ReceiverIterator<'a, T, E>
where T: Send, E: Send + Sync {
    fn next(&mut self) -> Option<T> {
        if self.blocking {
            self.reference.borrow().recv_block()
        } else {
            self.reference.borrow().recv()
        }
    }
}
