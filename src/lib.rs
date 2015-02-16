#![feature(core)]

use std::sync::{mpsc, RwLock};

#[cfg(test)]
mod test;

pub enum CommMsg<T, E> {
    Message(T),
    Error(E),
}

#[allow(dead_code)]
enum MaybeOwned<'a, A: 'a> {
    Owned(A),
    Borrowed(&'a A)
}

pub struct Sender<T, E> {
    closed: RwLock<bool>,
    inner: mpsc::Sender<CommMsg<T, E>>
}

pub struct Receiver<T, E> {
    closed: RwLock<bool>,
    error: RwLock<Option<E>>,
    inner: mpsc::Receiver<CommMsg<T, E>>
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
    let (tx, rx) = mpsc::channel();
    (Sender::from_old(tx), Receiver::from_old(rx))
}

impl <T, E> Sender<T, E> where T: Send, E: Send {
    pub fn from_old(v: mpsc::Sender<CommMsg<T, E>>) -> Sender<T, E> {
        Sender {
            closed: RwLock::new(false),
            inner: v
        }
    }

    pub fn into_inner(self) -> mpsc::Sender<CommMsg<T, E>> {
        self.inner
    }

    pub fn send(&self, t: T) -> Result<(), T> {
        match self.inner.send(CommMsg::Message(t)) {
            Ok(()) => Ok(()),
            Err(mpsc::SendError(CommMsg::Message(a))) => {
                * self.closed.write().unwrap() = true;
                Err(a)
            },
            Err(_) => unreachable!()
        }
    }

    pub fn send_all<I: Iterator<Item=T>>(&self, mut i: I) -> Result<(), (T, I)> {
        loop {
            match i.next() {
                None => break,
                Some(x) => {
                    match self.send(x) {
                        Ok(()) => {}
                        Err(x) => return Err((x, i))
                    }
                }
            }
        }
        Ok(())
    }

    pub fn close(self) { }

    pub fn error(self, e: E) -> Result<(), E> {
        match self.inner.send(CommMsg::Error(e)) {
            Ok(()) => Ok(()),
            Err(mpsc::SendError(CommMsg::Error(a))) => {
                * self.closed.write().unwrap() = true;
                Err(a)
            }
            Err(_) => unreachable!()
        }
    }

    pub fn is_closed(&self) -> bool {
        * self.closed.read().unwrap()
    }
}

impl <T, E> Clone for Sender<T, E> where T: Send, E: Send {
    fn clone(&self) -> Sender<T, E> {
        Sender {
            inner: self.inner.clone(),
            closed: RwLock::new(*self.closed.read().unwrap())
        }
    }
}

impl <T, E> Receiver<T, E> where T: Send, E: Send + Sync {
    pub fn from_old(v: mpsc::Receiver<CommMsg<T, E>>) -> Receiver<T, E> {
        Receiver {
            closed: RwLock::new(false),
            error: RwLock::new(None),
            inner: v
        }
    }

    pub fn into_inner(self) -> (mpsc::Receiver<CommMsg<T, E>>, Option<E>) {
        let error = self.error;
        let inner = self.inner;

        let mut error_guard = error.write().unwrap();
        (inner, error_guard.take())
    }

    pub fn recv(&self) -> Option<T> {
        if self.is_closed() {
            return None
        }
        match self.inner.try_recv() {
            Ok(CommMsg::Message(m)) => Some(m),
            Ok(CommMsg::Error(e)) => {
                * self.error.write().unwrap() = Some(e);
                * self.closed.write().unwrap() = true;
                None
            }
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                * self.closed.write().unwrap() = true;
                None
            }
        }
    }

    pub fn recv_block(&self) -> Option<T> {
        if self.is_closed() {
            return None
        }
        match self.inner.recv() {
            Ok(CommMsg::Message(m)) => Some(m),
            Ok(CommMsg::Error(e)) => {
                * self.error.write().unwrap() = Some(e);
                * self.closed.write().unwrap() = true;
                None
            }
            Err(mpsc::RecvError) => {
                * self.closed.write().unwrap() = true;
                None
            }
        }
    }

    pub fn has_error(&self) -> bool {
        self.error.read().unwrap().is_some()
    }

    pub fn take_error(&self) -> Option<E> {
        self.error.write().unwrap().take()
    }

    pub fn is_closed(&self) -> bool {
        * self.closed.read().unwrap()
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

impl <'a, T, E> Iterator for ReceiverIterator<'a, T, E>
where T: Send, E: Send + Sync {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.blocking {
            self.reference.borrow().recv_block()
        } else {
            self.reference.borrow().recv()
        }
    }
}
