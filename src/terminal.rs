pub(crate) struct WindowSize{
    pub(crate) cols: u16,
    pub(crate) rows: u16,
}

impl From<(u16,u16)> for WindowSize{
    fn from(dim: (u16,u16)) -> Self{
        Self{cols:dim.0,rows:dim.1}
    }
}
impl WindowSize{
    fn resize(&mut self, dim: (u16,u16)) {
        self.cols = dim.0;
        self.rows = dim.1;
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Position{
    pub(crate) r: u16,
    pub(crate) c: u16,
}

impl From<(u16,u16)> for Position{
    fn from(pos: (u16,u16))->Self{
        Self{r:pos.1,c:pos.0}
    }
}

impl From<Position> for (u16,u16){
    fn from(pos:Position)->(u16,u16){
        (pos.c as u16, pos.r as u16)
    }
}

impl Default for Position{
    fn default()->Self{
        Self{r:0,c:0}
    }
}

