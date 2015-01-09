use super::{Sender, Receiver, channel};

#[test]
fn basic() {
    let (sx, rx): (Sender<u64, ()>, Receiver<u64, ()>) = channel();

    sx.send(5u64).unwrap();
    sx.send(6u64).unwrap();
    sx.close();

    assert!(rx.recv() == Some(5u64));
    assert!(rx.recv() == Some(6u64));
    assert!(rx.recv() == None);
    assert!(rx.is_closed());
}

#[test]
fn error() {
    let (sx, rx) = channel();

    sx.send(5u32).unwrap();
    sx.error("hi".to_string()).unwrap();

    assert!(rx.recv() == Some(5u32));
    assert!(rx.recv() == None);
    assert!(rx.is_closed());
    assert!(rx.has_error());
    assert!(rx.take_error() == Some("hi".to_string()))
}

#[test]
fn iter() {
    let (sx, rx): (Sender<u32, ()>, Receiver<u32, ()>) = channel();

    sx.send(5u32).unwrap();
    sx.send(7).unwrap();
    sx.send(9).unwrap();

    let mut rx = rx.iter();

    let xs: Vec<u32> = rx.by_ref().collect();
    assert!(xs == vec![5,7,9]);

    sx.send(1).unwrap();
    sx.send(2).unwrap();
    sx.send(3).unwrap();

    let ys: Vec<u32> = rx.collect();
    assert!(ys == vec![1,2,3]);
}

#[test]
fn into_iter() {
    let (sx, rx): (Sender<u32, ()>, Receiver<u32, ()>) = channel();

    sx.send(5u32).unwrap();
    sx.send(7).unwrap();
    sx.send(9).unwrap();

    let mut rx = rx.into_iter();

    let xs: Vec<u32> = rx.by_ref().collect();
    assert!(xs == vec![5,7,9]);

    sx.send(1).unwrap();
    sx.send(2).unwrap();
    sx.send(3).unwrap();

    let ys: Vec<u32> = rx.collect();
    assert!(ys == vec![1,2,3]);
}

#[test]
fn iter_block() {
    // close()
    {
        let (sx, rx): (Sender<u32, ()>, Receiver<u32, ()>) = channel();

        sx.send(5u32).unwrap();
        sx.send(7).unwrap();
        sx.send(9).unwrap();
        sx.close(); // this close is required

        let rx = rx.blocking_iter();
        let xs: Vec<u32> = rx.collect();
        assert!(xs == vec![5,7,9]);
    }
    // error()
    {
        let (sx, rx): (Sender<u32, ()>, Receiver<u32, ()>) = channel();

        sx.send(5u32).unwrap();
        sx.send(7).unwrap();
        sx.send(9).unwrap();
        sx.error(()).unwrap(); // this error is required

        let rx = rx.blocking_iter();
        let xs: Vec<u32> = rx.collect();
        assert!(xs == vec![5,7,9]);
    }
}
