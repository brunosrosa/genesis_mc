use tokio::process::Child;

pub struct ProcessGuard {
    pub child: Option<Child>,
}

impl ProcessGuard {
    pub fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            // Chamada não bloqueante ao sistema operacional (SIGKILL no Linux, TerminateProcess no Windows)
            let _ = child.start_kill();
        }
    }
}
