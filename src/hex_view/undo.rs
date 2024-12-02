use crate::HexResult;

use super::HexView;

pub trait UndoOperation: Send + Sync {
    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    fn undo(&self, edit_state: &mut HexView) -> HexResult<()>;

    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    fn redo(&self, edit_state: &mut HexView) -> HexResult<()>;
}

pub struct UndoChangeByte {
    pub position: usize,
    pub old_caret_pos: usize,
    pub old_value: u8,

    pub new_caret_pos: usize,
    pub new_value: u8,
}

impl UndoChangeByte {
    pub fn new(position: usize, old_caret_pos: usize, old_value: u8, new_caret_pos: usize, new_value: u8) -> Self {
        Self {
            position,
            old_caret_pos,
            old_value,
            new_caret_pos,
            new_value,
        }
    }
}

impl UndoOperation for UndoChangeByte {
    fn undo(&self, edit_state: &mut HexView) -> HexResult<()> {
        let Some(buffer) = edit_state.buffer.as_mut() else {
            return Ok(());
        };
        buffer.set_byte(self.position, self.old_value);
        edit_state.cursor.position = self.old_caret_pos;
        Ok(())
    }

    fn redo(&self, edit_state: &mut HexView) -> HexResult<()> {
        let Some(buffer) = edit_state.buffer.as_mut() else {
            return Ok(());
        };
        buffer.set_byte(self.position, self.new_value);
        edit_state.cursor.position = self.new_caret_pos;
        Ok(())
    }
}
