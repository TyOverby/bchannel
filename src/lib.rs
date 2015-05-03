use std::sync::{mpsc, RwLock};
use std::cell::Cell;

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

/// The sending end of the channel.
pub struct Sender<T : Send, E : Send> {
    closed: Cell<bool>,
    inner: mpsc::Sender<CommMsg<T, E>>
}

/// The receiving end of the channel.
pub struct Receiver<T : Send, E : Send> {
    closed: Cell<bool>,
    errored: Cell<bool>,
    error: RwLock<Option<E>>,
    inner: mpsc::Receiver<CommMsg<T, E>>
}

/// An iterator over received items.
///
/// This struct can either own or have a reference to the receiver that
/// it gets its elements from.
///
/// This struct can either block when waiting for a message, or it can finish
/// early (and be reusable) when it runs out of messages in the queue.
pub struct ReceiverIterator<'a, T: Send + 'a, E: Send + 'a> {
    reference: MaybeOwned<'a, Receiver<T, E>>,
    blocking: bool
}

impl <'a, A> MaybeOwned<'a, A> {
    fn borrow<'b: 'a>(&'b self) -> &'b A  {
        match *self {
            MaybeOwned::Owned(ref a) => a,
            MaybeOwned::Borrowed(a) => a
        }
    }
}

/// Returns a Sender-Receiver pair sending messages of type T, and
/// can fail with an error of type E.
pub fn channel<T, E>() -> (Sender<T, E>, Receiver<T, E>)
where T: Send + 'static, E: Send + 'static{
    let (tx, rx) = mpsc::channel();
    (Sender::from_old(tx), Receiver::from_old(rx))
}

impl <T, E> Sender<T, E>
where T: Send + 'static, E: Send + 'static {
    /// Converts an old-stype Sender to a bchannel Sender.
    pub fn from_old(v: mpsc::Sender<CommMsg<T, E>>) -> Sender<T, E> {
        Sender {
            closed: Cell::new(false),
            inner: v
        }
    }

    /// Returns the old-style Sender that is containd inside this Sender.
    pub fn into_inner(self) -> mpsc::Sender<CommMsg<T, E>> {
        self.inner
    }

    /// Sends a message through the channel.  Returns `Ok(())` if the sending
    /// might succeed, and returns an Err with the message that you tried to
    /// send in the event that the sending surely failed.
    pub fn send(&self, t: T) -> Result<(), T> {
        match self.inner.send(CommMsg::Message(t)) {
            Ok(()) => Ok(()),
            Err(mpsc::SendError(CommMsg::Message(a))) => {
                self.closed.set(true);
                Err(a)
            },
            Err(_) => unreachable!()
        }
    }

    /// Tries to send all of the messages in an iterator.  Returns Ok(()) if the
    /// sending might succeed and returns Err with a tuple containing the message
    /// that failed to send, and the remaining iterator.
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

    /// Closes the sending end of the channel.
    pub fn close(self) { }

    /// Closes the sending end of the channel with an error.
    pub fn error(self, e: E) -> Result<(), E> {
        match self.inner.send(CommMsg::Error(e)) {
            Ok(()) => Ok(()),
            Err(mpsc::SendError(CommMsg::Error(a))) => {
                self.closed.set(true);
                Err(a)
            }
            Err(_) => unreachable!()
        }
    }

    /// Returns true if any message has failed to send.
    pub fn is_closed(&self) -> bool {
        self.closed.get()
    }
}

impl <T, E> Clone for Sender<T, E>
where T: Send + 'static, E: Send + 'static {
    fn clone(&self) -> Sender<T, E> {
        Sender {
            inner: self.inner.clone(),
            closed: Cell::new(self.closed.get())
        }
    }
}

impl <T, E> Receiver<T, E>
where T: Send + 'static, E: Send + 'static {
    /// Converts an old-style receiver to a bchannel receiver.
    pub fn from_old(v: mpsc::Receiver<CommMsg<T, E>>) -> Receiver<T, E> {
        Receiver {
            closed: Cell::new(false),
            errored: Cell::new(false),
            error: RwLock::new(None),
            inner: v
        }
    }

    /// Returns the old-style receiver along with the error.
    /// The error will be None unless this channel was closed by an error.
    pub fn into_inner(self) -> (mpsc::Receiver<CommMsg<T, E>>, Option<E>) {
        let mut error_guard = self.error.write().unwrap();
        (self.inner, error_guard.take())
    }

    /// Returns the next message asyncrhonously.
    ///
    /// * If there is a message in the channels queue, it is returned in `Some`.
    /// * If there is no message ready, None is returned.
    /// * If the channel is closed, None is returned.
    /// * If the channel is closed with an error, None is returned.
    pub fn recv(&self) -> Option<T> {
        if self.is_closed() {
            return None
        }
        match self.inner.try_recv() {
            Ok(CommMsg::Message(m)) => Some(m),
            Ok(CommMsg::Error(e)) => {
                * self.error.write().unwrap() = Some(e);
                self.closed.set(true);
                self.errored.set(true);
                None
            }
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.closed.set(true);
                None
            }
        }
    }

    /// Returns the next message in the channe.  This method will block
    /// until either a message arrives or the channel is closed
    /// (either regularly) or by an error.
    ///
    /// * If a message arrives, the message is returned inside of `Some`.
    /// * If the channel is closed, `None` is returned.
    /// * If the channel is closed with an error, `None` is returned.
    pub fn recv_block(&self) -> Option<T> {
        if self.is_closed() {
            return None
        }
        match self.inner.recv() {
            Ok(CommMsg::Message(m)) => Some(m),
            Ok(CommMsg::Error(e)) => {
                * self.error.write().unwrap() = Some(e);
                self.closed.set(true);
                self.errored.set(true);
                None
            }
            Err(mpsc::RecvError) => {
                self.closed.set(true);
                None
            }
        }
    }

    /// Returns true if the channel was closed with an error.
    pub fn has_error(&self) -> bool {
        self.errored.get()
    }

    /// Returns the error if the channel was closed with an error.
    /// This method moves the error out of the Receiver, so subsequent
    /// calls will return None.
    ///
    /// Returns `None` if the channel wasn't closed with an error, or if
    /// the error has already been taken.
    pub fn take_error(&self) -> Option<E> {
        self.errored.set(false);
        self.error.write().unwrap().take()
    }

    /// Returns true if the channel is closed.
    pub fn is_closed(&self) -> bool {
        self.closed.get()
    }

    /// Returns an iterator over the messages in this receiver.
    /// The iterator is non-blocking, and borrows this receiver.
    pub fn iter(&self) -> ReceiverIterator<T, E> {
        ReceiverIterator {
            blocking: false,
            reference: MaybeOwned::Borrowed(self)
        }
    }

    /// Returns an iterator over the messages in this receiver.
    /// The iterator is blocking and borrows this receiver.
    pub fn blocking_iter(&self) -> ReceiverIterator<T, E> {
        ReceiverIterator {
            blocking: true,
            reference: MaybeOwned::Borrowed(self)
        }
    }

    /// Returns an iterator over the messages in this receiver.
    /// The iterator is non-blocking and consumes this receiver.
    pub fn into_iter(self) -> ReceiverIterator<'static, T, E> {
        ReceiverIterator {
            blocking: false,
            reference: MaybeOwned::Owned(self)
        }
    }

    /// Returns an iterator over the messages in this receiver.
    /// The iterator is blocking, and consumes this receiver.
    pub fn into_blocking_iter(self) -> ReceiverIterator<'static, T, E> {
        ReceiverIterator {
            blocking: true,
            reference: MaybeOwned::Owned(self)
        }
    }
}

impl <'a, T, E> Iterator for ReceiverIterator<'a, T, E>
where T: Send + 'static, E: Send + 'static {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.blocking {
            self.reference.borrow().recv_block()
        } else {
            self.reference.borrow().recv()
        }
    }
}
