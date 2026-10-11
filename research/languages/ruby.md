# Ruby

## Snapshot
| | |
|---|---|
| First appeared | 1995 (0.95 released 21 Dec 1995; 1.0 in Dec 1996), Japan |
| Designed by | Yukihiro "Matz" Matsumoto; core team of committers; Matz retains final say on language design |
| Current version (Oct 2026) | 4.0.0 (25 Dec 2025); 4.0.7 (15 Sep 2026); 3.4.11 (23 Sep 2026); 4.1 expected on the usual 25 Dec cadence (not announced) |
| Governance | Matz as benevolent dictator for language design; Ruby core team for the implementation; Ruby Association (Japan) and Ruby Central (US, runs RubyGems.org and RubyConf); annual Christmas release |
| Typing | Dynamic, strong; everything is an object; optional external type signatures (RBS, since 3.0) checked by Steep or Sorbet |
| Memory | Mark-and-sweep GC with generational and incremental modes (since 2.1/2.2); compaction since 2.7 |
| Compilation | YARV bytecode interpreter (since 1.9); YJIT (Rust, production since 3.2); ZJIT (new, experimental in 4.0) |
| Primary domains | Web applications (Rails), DevOps tooling (Chef, Puppet, Vagrant, Homebrew), scripting |
| Notable users | GitHub, Shopify, Stripe, Airbnb, Basecamp/37signals, Square, Instacart, Zendesk |

Usage: Stack Overflow 2025 puts Ruby at 6.4% of all respondents and 6.9% of
professionals [1], down from roughly 10% a decade earlier; RubyGems hosts
175,000+ gems with 202+ billion downloads [2].

## Stated design goals

Matz's goal is unusual among language designers in being about the
programmer's feelings rather than the program's properties. On ruby-lang.org
he is quoted: "I wanted a scripting language that was more powerful than
Perl, and more object-oriented than Python" (2001), and he describes "trying
to make Ruby natural, not simple", adding that "Ruby is simple in appearance,
but is very complex inside, just like our human body" (2000) [3]. The widely
quoted formulation "I hope to see Ruby help every programmer in the world to
be productive, and to enjoy programming, and to be happy. That is the primary
purpose of Ruby language" is from Matz's conference talks (circa 2008; the
exact primary source was not pinned down in this research). The associated
"principle of least surprise" is, by Matz's own clarification, least surprise
*to Matz* after learning the language, not to a newcomer. Ruby blends Perl
(text processing, sigils for scope, regex literals), Smalltalk (everything is
an object, blocks, message sending), Lisp (closures; "in Ruby closures, I
wanted to respect the Lisp culture" [3]), Eiffel and Ada.

## Strengths

### Blocks, `yield` and internal iterators

`[1,2,3].each { |x| puts x }`, `map`, `select`, `inject`, `File.open(...) { |f| ... }`
and `Enumerable` (which gives roughly 50 methods to any class that defines
`each`) made passing a code block to a method the default way to loop, to
manage resources and to build DSLs. The trailing-block syntax is the direct
ancestor of Kotlin's trailing lambdas and Swift's trailing closures, Groovy
copied it wholesale, and Rust's and Kotlin's iterator adaptors (`map`,
`filter`, `fold`) follow `Enumerable`'s naming more than Lisp's. Internal
iteration (the collection calls you) is also why Ruby never needed a
generator keyword for the common case; `Enumerator` (1.9) provides external
iteration when needed. The evidence of success is imitation across five
mainstream languages and the fact that Rails' entire API surface is block-
based.

### Developer happiness as a measurable product: Rails and convention over configuration

Rails (DHH, 2004) turned Ruby's metaprogramming into "convention over
configuration": a model named `User` maps to table `users`, `has_many :posts`
generates a dozen methods, `rails generate` scaffolds a working application.
Rails 8.0 (7 November 2024) and 8.1 (22 October 2025) continue the theme with
Kamal deployment, Solid Queue/Cache/Cable on SQLite, and "no PaaS needed" as
the stated goal [4]. The influence is enormous: Django's admin, Laravel
(explicitly "Rails for PHP"), Phoenix (by former Rails contributors), Play,
Grails, ASP.NET MVC and Sails all copied the structure. The evidence that
"happiness" translates to productivity is the list of companies that reached
scale on one Rails monolith (GitHub, Shopify, Airbnb, Stripe's early years)
and Shopify's continued investment in Ruby performance rather than migration.

### Bundler and Gemfile: the model Cargo copied

Bundler (Yehuda Katz and Carl Lerche, 2010) introduced the pattern now
universal: a declarative manifest (`Gemfile`), a resolver producing a lockfile
(`Gemfile.lock`) committed to version control, deterministic installs from the
lock, and `bundle exec` to run inside the resolved environment. Katz and
Lerche then designed Cargo (2014) on the same shape, Cargo's manifest/lock
split was copied by npm (`package-lock.json`, 2017), Yarn, Go modules
(`go.sum`), Swift Package Manager, Python's Poetry/uv and finally PEP 751 [5].
This is one of the clearest cases in this research of a design proven by
universal imitation.

### A consistent object model

Every value is an object including integers, `nil`, classes and methods;
operators are method calls (`1.+(2)`); `method_missing`, `define_method`,
`send`, `instance_eval`, refinements and `include`/`extend`/`prepend` form a
small, uniform metaobject protocol. `Comparable` and `Enumerable` are the
reference design for "implement one method, get fifty", echoed by Rust's
trait default methods and Kotlin's extension functions.

### Pragmatic evolution: Ruby 2.x to 3.x without a Python-style schism

Ruby's one painful transition was 1.8 to 1.9 (2007-2011: new VM, string
encodings), absorbed over three years. Ruby 3.0 (25 December 2020) changed
keyword-argument semantics with warnings issued in 2.7 first; 3.4 (2024)
warns on mutation of string literals ahead of freezing them; 4.0 (2025)
removed `--rjit` and shipped no large incompatibilities [6]. The annual
Christmas release with a warn-then-break cycle is the practice Python later
adopted.

### Performance: YJIT shows a dynamic language can be made 2x faster in place

"Ruby 3x3" (Matz, 2015) targeted 3x over 2.0; MJIT (3.0) failed to deliver on
Rails workloads. YJIT (Shopify, Maxime Chevalier-Boisvert; merged in 3.1, Dec
2021; rewritten in Rust and production-ready in 3.2) uses lazy basic-block
versioning and reports substantial Rails speed-ups at Shopify and GitHub. ZJIT
in 4.0 is the next-generation design (method-based, Rust 1.85+), currently
"faster than the interpreter, but not yet as fast as YJIT", with production
use discouraged until 4.1 [6]. Shopify's Prism parser became the default in
3.4. The lesson parallels Python's: an in-place JIT buys roughly 1.5-2x; the
remaining cost is semantic.

## Weaknesses

### Metaprogramming and monkey patching: maintenance costs

Open classes let any file redefine `String#capitalize` or add `Integer#days`;
ActiveSupport does exactly that across hundreds of core methods. The costs:
behaviour depends on load order; two gems patching the same method conflict
silently; `method_missing`-based APIs (`find_by_name_and_email`) are invisible
to tools, documentation and type checkers; stack traces run through generated
methods. Fixes attempted: refinements (2.0, 2013) scope patches lexically but
are rarely used because they do not apply to `send`, blocks or most
metaprogramming paths; Rails replaced dynamic finders with `find_by(...)`;
Sorbet and RBS had to add `T.untyped` escape hatches and generated signatures
for Rails (Tapioca) precisely because the dynamic surface cannot be inferred.
This is the clearest evidence in the research that unrestricted open classes
trade short-term expressiveness for long-term tooling.

### Typing retrofits with limited adoption

RBS (separate `.rbs` signature files, Ruby 3.0, with Steep as checker and
TypeProf as inference tool) and Sorbet (Stripe, open-sourced 2019; inline
`sig { params(x: Integer).returns(String) }` with runtime checks) coexist, and
Matz has refused to put annotations in Ruby syntax. Shopify reports Sorbet
checking 99% of its 75,000 files with signatures on 71% of 1.5 million
methods [7], and in April 2025 added support for inline RBS comments to Sorbet
to reconcile the two formats [8]. No community-wide survey with an adoption
percentage was found in this research; outside Stripe, Shopify and a few large
monoliths, typed Ruby is uncommon, and the gem ecosystem ships few signatures.
Compared with TypeScript's 40% exclusive use and Python's 86% among typing-
survey respondents, Ruby is the counter-example: a retrofit with two competing
formats, no editor-driven adoption path and no checker in the default
toolchain, which stayed a large-company tool. The lesson is that gradual
typing adoption needs one format, inline syntax and default tooling.

### Concurrency: a GVL, and Ractors still experimental after five years

MRI has a Global VM Lock; threads give I/O concurrency but no CPU parallelism,
and the standard answer is multiple processes (Puma workers, Sidekiq
processes). Ractors (3.0, 2020) provide actor-style isolated parallelism with
deep-copy or move semantics for messages, but almost no gems are Ractor-safe
and Ruby 4.0 still labels them experimental, adding `Ractor::Port` and
`Ractor.shareable_proc` to fix message-passing ergonomics and reducing global
lock contention in internal data structures [6]. Fibers plus the Fiber
scheduler interface (3.0) and the `async` gem give colourless concurrency for
I/O without `await`, which is a genuinely attractive design, but the ecosystem
remained thread-and-process based. Five years of "experimental" is evidence
that bolting an isolation model onto a language whose standard library and
gems assume shared global state does not reach adoption.

### `nil` and `NoMethodError`

`undefined method 'x' for nil:NilClass` is the characteristic Ruby runtime
error; every method can return `nil` and nothing in the language marks it.
The safe-navigation operator `&.` (2.3, 2015) and `Array#dig`/`Hash#dig`
(2.3) are the fixes, and Sorbet's `T.nilable` is the typed fix. Ruby shows
that an elegant uniform object model does not remove the need for the type
system to know about absence.

### Ecosystem governance stress in 2025

In the second half of 2025 a dispute between Ruby Central and the long-time
RubyGems/Bundler maintainers over control of the GitHub organisations and
RubyGems.org operations led to maintainers being removed, a community fork
(gem.coop), and a subsequent arrangement in which the Ruby core team took
stewardship of the RubyGems and Bundler repositories. This sequence is
reported from memory of 2025 coverage and was not independently re-verified in
this research session; Ruby Central's 2026 posts refer to revised bylaws
(June 2026) and the Shopify-funded "Ruby Shield" supply-chain programme [9].
The lesson stands either way: the package registry and installer are critical
infrastructure whose governance must be written down before a crisis.

## Designer retrospectives

Paraphrased from keynotes and posts; the first three are from memory and were
not re-verified against transcripts in this session.

- Matz regrets not having a namespace-isolation story earlier; Ruby Box
  (experimental in 4.0, `RUBY_BOX=1`) is the attempted fix [6].
- Matz has called the 2.x keyword-argument/hash ambiguity a design error,
  fixed with 2.7 warnings and a 3.0 break.
- Matz does not want type annotations in Ruby source; separate RBS files and
  TypeProf inference are his path, and he has acknowledged the adoption cost.
- The YJIT team documented that MJIT's method-at-a-time C compilation was the
  wrong design for Ruby and that lazy basic-block versioning in Rust was the
  fix; ZJIT is the next revision [10].
- Koichi Sasada (Ractors) has written that making the standard library and C
  extensions Ractor-safe was the underestimated cost.

## Influence

Copied by others: trailing blocks (Kotlin, Swift, Groovy); `Enumerable`-style
collection APIs (Rust, Kotlin); Bundler's manifest+lockfile (Cargo, npm, Go,
Swift PM, uv); Rails conventions (Django admin, Laravel, Phoenix, Grails);
`#{}` interpolation and `unless` (Crystal, Elixir, CoffeeScript); RSpec's DSL
style (Jasmine, Mocha, Kotest). Crystal is Ruby's syntax with static typing;
Elixir's syntax was chosen by a former Rails core member to be Ruby-like.
Copied from others: Perl (sigils for scope, regex literals), Smalltalk
(blocks, everything-is-an-object), Lisp (closures), Python (iterators,
keyword arguments), CLU (yield), Erlang (Ractors as actors).

## Lessons for Sisu

- **Adopt**: trailing-block syntax for closures passed to methods; a rich
  mixin-style collection protocol where implementing one method yields the
  rest; manifest + lockfile dependency management with a single tool shipped
  with the language; annual releases with a warn-then-break deprecation
  cycle; uniform "everything is a value with methods" semantics for operators.
- **Adapt**: metaprogramming through declared, compile-time mechanisms (derive
  macros, annotations) rather than open classes and `method_missing`, so the
  Rails-style convention-over-configuration experience survives static
  analysis; symbols as a real enum/interned-identifier type rather than a
  parallel string universe; fibers/colourless I/O concurrency as a model, but
  with isolation designed into the standard library from the start so the
  Ractor "experimental for five years" outcome is avoided.
- **Avoid**: global monkey patching of core types; `nil` as a universal
  member of every type; two competing type-signature formats with none in the
  default toolchain; a VM lock whose removal depends on auditing every
  library; Perl-style special variables.
- **Open questions raised**: whether "developer happiness" can be a stated
  design goal with any objective check, or whether it is a proxy for
  readability plus conventions; whether a language that allows open classes
  can ever be statically typed well enough for adoption, or whether Ruby's
  typing stall is inherent; how much of Rails' productivity came from Ruby
  the language versus Rails the conventions (Laravel and Django suggest the
  conventions travel).

## Sources

1. Stack Overflow Developer Survey 2025, Technology. https://survey.stackoverflow.co/2025/technology (2025)
2. RubyGems.org statistics via Wikipedia, "RubyGems". https://rubygems.org/stats ; https://en.wikipedia.org/wiki/RubyGems (retrieved Oct 2026)
3. ruby-lang.org, "About Ruby" (Matz quotes). https://www.ruby-lang.org/en/about/ (current)
4. Ruby on Rails release announcements for 8.0 (7 Nov 2024) and 8.1 (22 Oct 2025). https://rubyonrails.org/category/releases
5. Bundler history (Katz, Lerche, 2010); Cargo's acknowledgement of Bundler's design in its early documentation and Yehuda Katz's posts (2014); PEP 751 (2025)
6. "Ruby 4.0.0 Released" (ZJIT, Ractor::Port, Ruby Box, rjit removal). https://www.ruby-lang.org/en/news/2025/12/25/ruby-4-0-0-released/ (25 Dec 2025); ruby-lang.org news index for 4.0.x and 3.4.x releases (2026)
7. Shopify Engineering, "Adopting Sorbet at Scale" / static typing posts (99% of 75,000 files, 71% of 1.5M methods). https://shopify.engineering/static-typing-ruby
8. Rails at Scale, "RBS support for Sorbet". https://railsatscale.com/2025-04-23-rbs-support-for-sorbet/ (23 Apr 2025)
9. Ruby Central news (Ruby Shield reflection, Sept 2026; bylaws June 2026). https://rubycentral.org/news/ (2026)
10. Maxime Chevalier-Boisvert, "YJIT: Building a New JIT Compiler for CRuby" (RubyKaigi 2021) and Shopify engineering posts on YJIT (2021-2024); Ruby 3.2 and 3.3 release notes. https://www.ruby-lang.org/en/news/
11. Sorbet, "TypeScript to Sorbet" and docs. https://sorbet.org/docs/from-typescript ; RBS repository https://github.com/ruby/rbs
12. "The state of static typing in Ruby in 2025" (DEV community summary of the RBS/Sorbet landscape). https://dev.to/aeremin/the-state-of-static-typing-in-ruby-in-2025-3o4b (2025)
