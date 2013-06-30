use bits;
use gzip::header;

pub trait MemberReceiver<R: bits::recv::Receiver<u8>> {
  pub fn receive_member(&mut self, hdr: &header::Header) -> ~R;
  pub fn finish_member(&mut self, hdr: ~header::Header, recv: ~R);
}

impl MemberReceiver<~[u8]> for ~[(~header::Header, ~[u8])] {
  fn receive_member(&mut self, _hdr: &header::Header) -> ~~[u8] {
    ~~[]
  }

  fn finish_member(&mut self, hdr: ~header::Header, recv: ~~[u8]) {
    self.push((hdr, *recv));
  }
}

impl MemberReceiver<()> for () {
  fn receive_member(&mut self, _hdr: &header::Header) -> ~() {
    ~()
  }

  fn finish_member(&mut self, _hdr: ~header::Header, _recv: ~()) {
  }
}
