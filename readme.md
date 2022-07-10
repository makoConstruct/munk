`munk` is a lisplike that was made as a query language for [neschat](https://github.com/dream-shrine/neschat/).

`music_query_and_hello.munk` (written in [termpose](https://github.com/makoConstruct/termpose)):
```
param input
use (neschat publish latest_version articles_by_endorsement)

publish (input hello_message)

let from (latest_version (input me

articles_by_endorsement 
    max_hops 5
    max_results 25
    tag good_music
    age 7days
```

```rust
let music_account:OID = ...;
let query_munk:Wood = wood::parse_multiline_termpose(fs::read_to_string("music_query_and_hello.munk")?)?;
let query_input = woods![
    woods!["hello_message", nes.sign(music_account, Wood::leaf("hello world".into()))],
    woods!["me", Base64Bi.woodify(music_account)],
];
let good_music_as_of_this_week:Wood = server.query_with_input(&query_munk, &query_input).await;
```

## Why did neschat want a query language?

To make a client as responsive as possible, it's important that servers provide all of the objects needed to render a page with just one request. For complex apps, there are two ways of doing this. One way is to have the server already know what the client needs when it's rendering different kinds of views and pages. For large or plural client ecosystems, this is untennable. It means that server APIs will have to keep growing more and more complex, and changes to clients will have to wait on for servers to update before the changes can become active.

Another way of supporting one-request page loads is by allowing the client to just explain fully what they need to the server in their own words. Sometimes these expressions might be complex. Some of their needs might be contingent on things that the server knows that the client doesn't yet know. Concretely: They might want objects that they only know the IDs of after doing another query leading up to it. For instance, a common situation in neschat: There's a document with a hyperlink to an object, but it's a link to an older version. You want to see the latest version, with replies and annotations and issues that backlink to it. You want to see `replies(latest_version(object))`. To support explaining that kind of dependency resolution to the server, the query language basically need a programming language.

For a query language, I considered using wasm, or racket, or clojure, or deno. I was a little worried that wasm might have unpredictably large (and totally uncontrollable) compilation sizes. As for the others, I wanted to use a language runtime that's brutally simple enough to ensure that there were absolutely zero security risks in running scripts that had been sent to the server from rando clients. So I decided to just write one myself. I then realized that the thing I'd written was kinda neat and got excited about it.

I was going to give it implicit parameters, and have no real distinction between macros and functions. I'm developing a sense for why most languages don't do these things, but it's fun to have a chance to try it.

## Why didn't you finish this? (It doesn't even compile!)

It occurred to me that memory management was going to be a little bit more complicated than I thought, and it ended up taking too long. Refcounting wouldn't have worked, because it would eventually end up enabling a malicious client to leak memory on the server (I couldn't easily prove to myself that this wasn't already possible). GC would have taken a while to integrate. Arena bump allocation would have worked fine, but it's so crude, it would have drained away a lot of the enthusiasm I had for making `munk`.

I wasn't actually totally sure that having a query language would speed us up with making the prototype. So I decided to try making the prototype without one. A prototype exists to be burned anyway...

I might return to this, but we'll see.