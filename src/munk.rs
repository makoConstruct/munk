use simple_error;
use std::{
    any::{Any, TypeId},
    collections::hash_map::{HashMap, Entry::{Vacant, Occupied}},
    iter::Peekable,
};
use wood::{to_woodslist, woods, Wood, L, B};

/// A munk program consists of an evaluator that turns wood into obs and then the ob is stringified and that's what's sent back to the user.

/// TODO use string interning to make hashing against a string and comparing equality quicker for fast symbol lookup and reduce the amount of copying that needs to be done

//If you need mutability, use interior mutability. It's okay to return an error when you don't have it.
//HARD TODO: wait, this is untennable, this will allow the user to create memory leaks

type Res = Result<(), Box<dyn Error>>;

type Rob = Rc<dyn Ob>;

trait Ob {
    fn call(&self, s: &Rob, vm: &mut MunkVM, w: &Wood) -> Rob {
        s.clone()
    }
    /// The unit doesn't really mean much. It doesn't have to be accurate. The typical limit will be... 20_000.
    //TODO remove default impl
    fn eval_cost_upper_bound(&self, vm: &MunkVM, w: &Wood) -> f32 { 0.0 }
    fn test_truth(&self) -> bool {
        false
    }
    fn serialize(&self) -> Wood;
    fn as_any(&self)-> &dyn Any;
    /// locks it
    fn as_any_mut(&self)-> Option<&mut dyn Any> { None }
}
impl dyn Ob {
    fn assume<T>(&self)-> &T {
        self.as_any().downcast_ref().unwrap()
    }
}
fn ok(v:Rob)-> Result<Rob, Rob> {
    if v.as_any().is::<ErrorOb>() { Ok(v) } else { Err(v) }
}


#[derive(Clone, Eq, PartialEq, Hash)]
struct WoodOb(&Wood);
impl Ob for WoodOb {
    fn eval_cost_upper_bound(&self) -> f32 {
        0.0
    }
    fn test_truth(&self) -> bool {
        self.0.contents().len() != 0
    }
    fn serialize(&self) -> Wood {
        self.0.clone()
    }
    fn as_any(&self)-> &dyn Any { self }
}

type FnFn = fn(sid: &Rob, &mut MunkVM, &Wood) -> Rob;
type RuntimeBoundFn = fn(sid: &Rob, &mut MunkVM, &Wood) -> f32;
struct FunctionOb {
    eval: FnFn,
    runtime_bound: RuntimeBoundFn,
    name: String,
}
impl Ob for FunctionOb {
    fn eval_cost_upper_bound(&self, vm: &MunkVM, w: &Wood) -> f32 {
        self.runtime_bound(w)
    }
    fn serialize(&self) -> Wood {
        Wood::leaf(format!("function:{}", &self.name))
    }
    fn as_any(&self)-> &dyn Any { self }
}

struct NullOb;
impl Ob for NullOb {
    fn eval_cost_upper_bound(&self, vm: &MunkVM, w: &Wood) -> f32 {
        0.0
    }
    fn serialize(&self) -> Wood {
        Wood::leaf("null".into())
    }
    fn as_any(&self)-> &dyn Any { self }
}

struct ErrorOb {
    line_and_col: (isize, isize),
    message: String,
}
impl Ob for ErrorOb {
    fn eval_cost_upper_bound(&self, vm: &MunkVM, w: &Wood) -> f32 {
        0.0
    }
    fn serialize(&self) -> Wood {
        woods![
            "Error",
            woods!["line", self.line.woodify()],
            woods!["column", self.column.woodify()],
            woods!["message", self.message.clone()]
        ]
    }
    fn as_any(&self)-> &dyn Any { self }
}

fn error_at(w:&Wood, s:String)-> Rob {
    Rob::new(ErrorOb{line_and_col: w.line_and_col(), message: s})
}

struct ModOb {
    name: String,
    contents: HashMap<String, Rob>,
}
impl Ob for ModOb {
    fn call(&self, s: &Rob, vm: &mut MunkVM, w: &Wood) -> Rob {
        let cm = s.clone();
        match w.what() {
            L(v)=> {
                s.clone()
            }
            B(v)=> {
                let mut mso = s.clone();
                for nv in v.tail() {
                    let key = match nv.what() {
                        L(s) => {
                            s
                        }
                        B(v)=> {
                            let r = vm.eval(nv);
                            if let Some(&WoodOb(Leafv(Leaf{ref v, ..}))) = r.as_any().downcast_ref() {
                                v
                            }else{
                                return Self::error_at(nv, "runtime error: the result of this evaluation was not a string");
                            }
                        }
                    };
                    let ms = if let Some(m) = mso.as_any().downcast_ref::<ModOb>{
                        m
                    } else {
                        return error_at(nv, format!("attempting to module index a non-module"))
                    };
                    if let Some(no) = ms.contents.get(key) {
                        if no.as_any().is::<ModOb>() {
                            mso = no.clone();
                        }else{
                            return error_at(nv, format!("no such module as {} here", key))
                        }
                    }
                }
                mso
            }
        }
    }
    /// The unit doesn't really mean much. It doesn't have to be accurate. The typical limit will be... 20_000.
    fn eval_cost_upper_bound(&self, vm: &MunkVM, w: &Wood) -> f32 {
        w.len() as f32
    }
    fn test_truth(&self) -> bool {
        false
    }
    fn serialize(&self) -> Wood;
    fn as_any(&self)-> &dyn Any { self }
}
//TODO: Error (and error propagation logic in blocks), Or should it be "special return" types.
// Language features to do: Catch errors,
//think about dynamic dispatch/dyn methods? I don't need it yet though. (omg what if dynamic dispatches were called via woods rather than strings o-o)

struct Scope {
    namespace: HashMap<String, Rob>,
}
struct MunkVM {
    /// a module, contains other modules, which can be grafted to the namespace with `use`
    external: Rob,
    scopes: Vec<Scope>,
}

const NULL:Rob = Rob::new(NullOb);

/// for the "use" syntax
fn use_recurse(ext_mods:&HashMap<String, Rob>, namespace:&mut HashMap<String, Rob>, on:&Wood)-> Result<(), Rob> {
    match uw.what() {
        L(v)=> {
            //terminate, make the graft
            let mr = ext_mods.get(v).ok_or_else(|| error_at(uw, "module item not found"))?;
            namespace.insert(v, mr.clone());
        }
        B(v)=> {
            if v.len() == 0 {
                return Err(error_at(uw, "syntax error: empty branch in use"));
            }
            if v.len() == 1 {
                return Err(error_at(uw, "syntax error: this use branch doesn't import anything. Try removing the parens"));
            }
            let mod_name = v[0].get_leaf().ok_or_else(|| error_at(v[0], "this should be the name of the module being imported from".into()))?;
            let moduleo = ext_mods.get(mod_name).ok_or_else(|| error_at(v[0], "no such module"))?;
            let mo = moduleo.as_any().downcast_ref::<ModOb>().ok_or_else(|| error_at(v[0], "syntax error: This item is not a module. It can be imported, but it cannot be imported from"))?;
            for sm in v[1..] {
                use_recurse(&mo.contents, namespace, sm)?;
            }
        }
    }
    Ok(())
}

impl<'a> MunkVM<'a> {
    fn def_ob(&mut self, name: &str, v: Rob) {
        self.stack.last().unwrap().insert(name, v);
    }
    /// for function lines
    fn invoke(&mut self, line: &Wood) -> Rob {
        if let Some(head) = line.seek_head() {
            if let Some(name) = head.get_leaf() {
                if let Some(f) = self.scopes.iter().rev().find_map(|s| s.namespace.get(name)) {
                    f.call(self, line)
                } else {
                    error_at(format!("There's no function called **{}** in scope", name))
                }
            } else {
                self.invoke(head)
            }
        }else{
            error_at(line, "A leaf wood was invoked as a function??".into())
        }
    }
    fn decompose_single_if(&self, line: &Wood) -> Result<(&Wood, &[&Wood], Option<&[&Wood]>), Rob> {
        let lc = line.contents();
        if lc.len() == 4 {
            let else_block = lc.last().unwrap();
            if else_block.initial_str() == "else" {
                //then it's the (if cond (do ...) (else ...))
                let cond = lc[1];
                let doings = lc[2].tail();
                let elsings = lc[3].tail();
                return Ok((cond, doings, Some(elsings)));
            }
        }
        //it has to be a (if cond ...) then
        let (cond, doings) = decompose_simple_if_affirmative(line)?;
        Ok((cond, doings, None))
    }
    fn eval(&mut self, line: &Wood) -> Rob {
        match line.initial_str() {
            "#"=> { /*do nothing, it's a comment*/ }
            "use" => {
                let MunkVM{ref external, ref mut namespace, ..} = *self;
                for uw in line.tail() {
                    if let Some(e) = use_recurse(external, namespace, uw).err() {
                        return e;
                    }
                }
                NULL.clone()
            }
            "do" => {
                self.stack.push(Scope {
                    namespace: HashMap::new(),
                });
                let r = self.run_block(line.tail(), true);
                self.stack.pop();
                r
            }
            "if" => {
                match self.decompose_single_if(line) {
                    Ok(t)=> evaluate_conditional(t),
                    Err(e)=> e
                }
            }
            "def"=> {
                let mut li = line.tail().iter();
                if let Some(name) = li.next() {
                    let ns = &mut self.stack.last().unwrap().namespace;
                    match ns.entry(name) {
                        Ok(_)=> {
                            error_at(format!("runtime error: there's already a variable in this scope called {}", name))
                        }
                        Err(v)=> {
                            v.insert(NULL.clone());
                            NULL.clone()
                        }
                    }
                }else{
                    error_at("syntax error: def block with no name")
                }
            }
            /// same as def but allows functions to silently grab it as an implicit parameter (which is convenient but often very tricksy, imagine a function changing to a new version ) (for now, def also allows that. Watch out! :]]])
            // "provide"=> {}
            _ => {
                self.invoke(line)
            }
        }
    }

    fn run_block(&mut self, mut lines: impl Peekable<Item = &Wood>, except_errors: bool) -> Rob {
        fn decompose_block_if(
            line: &Wood,
            lines_iter: impl Peekable<Item = &Wood>,
        ) -> Result<(&Wood, &[&Wood], Option<&[&Wood]>), Rob> {
            if let Some(next_block) = lines_iter.peek() {
                if next_block.initial_str() == "else" {
                    //then it's definitely the (if cond ...) (else ...) form
                    let (condw, doing) = decompose_simple_if_affirmative(line)?;
                    let elseb = next_block.tail();
                    //advance the iter
                    lines_iter.next();
                    return Ok((condw, doing, Some(elseb)));
                }
            }
            decompose_single_if(line)
        }

        fn decompose_simple_if_affirmative(ifw: &Wood) -> Result<(&Wood, &[&Wood]), Rob> {
            let lc = line.contents();
            if lc.len() < 2 {
                return Err(
                    error("syntax error: a **if** without a condition or any consequences")
                );
            } else if lc.len() < 3 {
                return Err(error("syntax error: a **if** without any consequences"));
            }
            //we know the first is just "if"
            let condw = lc[2];
            let doing = lc[2..];
            Ok((condw, doing))
        }

        if !lines.peek().is_some() {
            return error("syntax error: empty block");
        }
        while let Some(line) = lines.next() {
            let r: Rob = match line.initial_str() {
                // handle the non-atomic expressions that can occur in blocks
                "if" => {
                    //this will either be the (if cond ...) (else ...), or it may be just (if cond ...), or it may be (if cond (do ...) (else ...)). Sort that out.
                    let (cond, do_block, else_block) = match decompose_block_if(line, &mut lines) {
                        Ok(t) => evaluate_conditional(t),
                        Err(er) => return er,
                    };
                }
                // that's it for now... (eventually there will be try+catch pairs)
                _ => self.eval(line),
            };

            if except_errors && r.type_id() == Error::type_id() {
                return r;
            }
            if !bws.peek().is_some() {
                //then this is the final one, and so this is the one that gets returned
                return r;
            }
        }
    }
}

fn run(w: &Wood) -> String {
    let mut vm = MunkVM {
        root: w,
        scopes: Vec::new(),
        external: HashMap::new(),
    };
    
    vm.run_block(w.contents(), true).serialize()
}

type RustFn = fn(&Rob, &mut MunkVM, &Wood) -> Result<(), Rob>;
type RustRuntimeBound = fn(&Rob, &mut MunkVM, &Wood) -> f32;
impl MunkVM {
    fn get_or_intern(&mut self, v: &str) -> Si {
        let next_id = self.interned_strings_by_id.len();
        match self.interned_strings.entry(v) {}
        or_insert_with(|| (v.clone(), next_id))
    }
    fn register_function(&mut self, path:&[&str], f:RustFn, runtime_bound:RustRuntimeBound)-> Res<()> {
        let mut im = &mut self.external;
        if path.len() < 1 {
            bail!("the path of a munk function cannot be empty");
        }
        for n in path[..-1] {
            im = match im.entry(n) {
                Vacant(s)=> {
                    s.insert(ModOb{name: n.into(), contents: HashMap::new()}).as_any().downcast::<ModOb>().unwrap();
                }
                Occupied(s)=> {
                    s.get_mut()
                }
            }
        }
        let name = path.last().unwrap();
        im.insert(name, Rob::new(FunctionOb{eval: |s: &Rob, vm: &mut MunkVM, w: &Wood| f(s, vm, w).unwrap_or_else(|e| e), runtime_bound, name:name.into()}));
    }
}


/// A macro for type checking, fetching and translating ob types into rust types for munk function calls. Returns a result. Errors with an ErrorOb if requirements are not met.
macro_rules! bind_params {
    ($vm:expr, $w:expr, $i:ident : $t:ty, $($e:tt),*)=> {
        let $ident:$t = {
            let ob = if let Some(kvw) = $w.seek(stringify!($i)) {
                ok($vm.eval(valw))?
                // wait, we don't need to do this, it's the rust API
                // ok($vm.eval(pi)).map(|r|{
                //     if let VacantEntry(e) = vm.scope.entry() {
                //         e.insert($ident)
                //     }else{
                //         return Err(error_at($w, "duplicate parameter".into()));
                //     }
                // });
                $vm.seek_var(stringify!($ident)).ok_or_else(|| error_at($w, "the parameter wasn't passed"))?
            }else{
                return Err(error_at($w, "the parameter wasn't passed"));
            }
            ob.nativize()?
        };
        bind_params($vm, $w, $($e),*)
    }
}


fn main()-> Res<()> {
    let mut vm = MunkVM::new();
    vm.register_function(&["std", "print"], |s:&Rob, vm: &mut MunkVM, w: &Wood|{
        bind_params!(vm, w, message:String);
        println!("{}", &message);
        Ok(NULL.clone())
    }, |_,_,_| 1.0);
    let program = wood::parse_multiline_termpose(&std::fs::read_into_string("simple.munk"))?;
    println!("{}", &vm.run_block(program.contents(), true).serialize().to_woodslist());
    Ok(())
}