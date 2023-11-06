mod transpile_type;

trait Transpileable {
    fn transpile(self) -> String;
}
