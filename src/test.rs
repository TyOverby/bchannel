use super::{Sender, Receiver, channel};

#[test]
fn basic() {
    let (sx, rx): (Sender<uint, ()>, Receiver<uint, ()>) = channel();

    sx.send(5u).unwrap();
    sx.send(6u).unwrap();
    sx.close();

    assert!(rx.recv() == Some(5u));
    assert!(rx.recv() == Some(6u));
    assert!(rx.recv() == None);
    assert!(rx.is_closed());
}

#[test]
fn error() {
    let (sx, rx) = channel();

    sx.send(5u).unwrap();
    sx.error("hi".to_string()).unwrap();

    assert!(rx.recv() == Some(5u));
    assert!(rx.recv() == None);
    assert!(rx.is_closed());
    assert!(rx.has_error());
    assert!(rx.take_error() == Some("hi".to_string()))
}

#[test]
fn iter() {
    let (sx, rx): (Sender<uint, ()>, Receiver<uint, ()>) = channel();

    sx.send(5u).unwrap();
    sx.send(7).unwrap();
    sx.send(9).unwrap();

    let mut rx = rx.iter();

    let xs: Vec<uint> = rx.collect();
    assert!(xs == vec![5,7,9]);

    sx.send(1).unwrap();
    sx.send(2).unwrap();
    sx.send(3).unwrap();

    let ys: Vec<uint> = rx.collect();
    assert!(ys == vec![1,2,3]);
}

#[test]
fn into_iter() {
    let (sx, rx): (Sender<uint, ()>, Receiver<uint, ()>) = channel();

    sx.send(5u).unwrap();
    sx.send(7).unwrap();
    sx.send(9).unwrap();

    let mut rx = rx.into_iter();

    let xs: Vec<uint> = rx.collect();
    assert!(xs == vec![5,7,9]);

    sx.send(1).unwrap();
    sx.send(2).unwrap();
    sx.send(3).unwrap();

    let ys: Vec<uint> = rx.collect();
    assert!(ys == vec![1,2,3]);
}

#[test]
fn iter_block() {
    // close()
    {
        let (sx, rx): (Sender<uint, ()>, Receiver<uint, ()>) = channel();

        sx.send(5u).unwrap();
        sx.send(7u).unwrap();
        sx.send(9u).unwrap();
        sx.close(); // this close is required

        let mut rx = rx.blocking_iter();
        let xs: Vec<uint> = rx.collect();
        assert!(xs == vec![5,7,9]);
    }
    // error()
    {
        let (sx, rx): (Sender<uint, ()>, Receiver<uint, ()>) = channel();

        sx.send(5u).unwrap();
        sx.send(7u).unwrap();
        sx.send(9u).unwrap();
        sx.error(()).unwrap(); // this error is required

        let mut rx = rx.blocking_iter();
        let xs: Vec<uint> = rx.collect();
        assert!(xs == vec![5,7,9]);
    }
}
