# Perl and PHP

Two languages treated together because they are the canonical cautionary
tales of the scripting era, and because each also demonstrates something that
later languages failed to match: Perl's text-processing ergonomics and CPAN,
PHP's deployment model and its disciplined late-life modernisation.

## Snapshot
| | Perl | PHP |
|---|---|---|
| First appeared | 1987 (Larry Wall); Perl 5 in 1994 | 1995 (Rasmus Lerdorf, "Personal Home Page Tools"); PHP 3 in 1998 (Zend engine by Suraski and Gutmans); PHP 5 in 2004 |
| Designed by | Larry Wall; since 2020 a Perl Steering Council | Lerdorf, Andi Gutmans, Zeev Suraski; now the php-src internals group via RFCs; PHP Foundation (2021) funds core developers |
| Current version (Oct 2026) | 5.44.0 (July 2026); 5.42 (July 2025) | 8.5 (20 Nov 2025); 8.6 scheduled for GA 19 Nov 2026 (RC phase as of October) |
| Governance | Perl Steering Council elected by core; features via "perl5-porters" and experimental feature flags; sister language Raku (formerly Perl 6) separate since 2019 | RFCs on wiki.php.net voted by core contributors, 2/3 supermajority required since 2019; release managers per version; yearly release in November |
| Typing | Dynamic, weak with context sensitivity (scalar/list/void); no static types; `class` feature since 5.38 | Dynamic, weak with coercion; gradual declared types since 7.0 (scalar), 7.4 (properties), 8.0 (union), 8.1 (enums, readonly), 8.2 (DNF types); `declare(strict_types=1)` per file |
| Memory | Reference counting (cycles leak unless weakened) | Reference counting with cycle collector (since 5.3); request-scoped arena freed per request |
| Compilation | Interpreter over an op tree; no JIT | Zend VM bytecode; opcache; JIT since 8.0 (limited gains for web workloads) |
| Primary domains | System administration, text processing, bioinformatics, legacy web (CGI) | Server-side web: WordPress, Drupal, Laravel, Symfony, Magento, Facebook's origin |
| Notable users | Booking.com, DuckDuckGo (historically), BBC, bioinformatics pipelines, Debian tooling | Facebook (via Hack/HHVM), Wikipedia, WordPress.com, Slack (early), Etsy, Mailchimp |

Usage: Stack Overflow 2025: PHP 18.9% of all respondents (19.1% of
professionals), Perl 3.8% (3.4%) [1]. W3Techs reports PHP serving roughly
71-72% of websites whose server-side language is known (71.8% in March 2026),
with PHP 8 at about 57% and PHP 7 at about 34% of PHP sites (secondary
reporting of W3Techs figures) [2].

## Stated design goals

**Perl.** Wall's slogans: "There's more than one way to do it" (TIMTOWTDI) and
"Easy things should be easy and hard things should be possible" [3]. Wall, a
linguist, designed Perl to work like a natural language: context sensitivity,
redundancy, pronoun-like defaults (`$_`), and a large vocabulary. The goal was
to replace shell + sed + awk + C for system administration and report
generation ("Practical Extraction and Report Language").

**PHP.** Lerdorf has said repeatedly that he never intended to write a
language; PHP was a set of CGI tools for his home page that grew by accretion,
and its goal was to make it trivial to put dynamic content in HTML and deploy
by copying a file to a web server. The implicit design goal, stated later by
the core team, was "do not break the 70% of the web that runs on us", which
the RFC process now enforces.

## Strengths

### Perl: regular expressions as a language feature

Perl made regex a literal (`/.../`), added `=~`, named captures, `s///e`,
lookaround, non-greedy quantifiers, `/x` extended mode and Unicode properties.
The syntax was copied by PCRE (1997), hence by PHP, Nginx, Apache, Ruby, Java's
`java.util.regex`, JavaScript, Python's `re`, Rust's `regex` crate and .NET.
"Perl-compatible" became the definition of a regex dialect. This is the
strongest example of adoption-and-imitation evidence in this group. The
downside, also copied everywhere, is the write-only density of complex
patterns; Raku's grammars and Rust's `regex` (no backtracking, linear time) are
the attempted corrections.

### Perl: CPAN as the first great package ecosystem

CPAN (1995) established the pattern of a central index, mirrored archives,
`Makefile.PL`/`Build.PL` build conventions, `cpan`/`cpanm` installers, the
`Test::More` TAP testing protocol with CPAN Testers running every
distribution's tests across platforms and Perl versions, and POD documentation
rendered on the index. PyPI, RubyGems, CTAN-style archives and npm all
followed it; TAP (Test Anything Protocol) is still used outside Perl. Its
weakness, no lockfile and no built-in dependency isolation (`local::lib` and
`Carton` came late), is what Bundler fixed.

### PHP: a deployment model that "just works"

A PHP page is a file; copy it to a directory served by Apache's `mod_php` or
PHP-FPM and it runs, with a fresh process state per request (shared-nothing),
no daemon to manage, no build step, and failure isolated to one request. This
is why PHP powers most of the web by site count: the cheapest hosting on earth
supports it, and WordPress (first released 2003) could be installed by
non-programmers. Serverless functions (AWS Lambda, 2014) re-invented the
request-scoped, no-daemon model that PHP had in 1998. The costs (no persistent
state, per-request bootstrapping, difficulty with WebSockets) were later
addressed with opcache, Swoole, FrankenPHP and PHP 8.1 Fibers.

### PHP: late-life professionalisation through the RFC process

PHP 7.0 (December 2015) rebuilt the engine ("phpng") and was advertised as up
to twice as fast as 5.6 with half the memory, a response to Facebook's HHVM
(2014) threatening to fork the ecosystem [4]. Since then a yearly cadence has
delivered: scalar type declarations and return types (7.0), nullable types and
`void` (7.1), typed properties and arrow functions (7.4), union types, named
arguments, `match`, attributes, the nullsafe operator, constructor property
promotion and a JIT (8.0, 2020), enums, `readonly` properties, fibers and
intersection types (8.1), readonly classes (8.2), typed class constants (8.3),
property hooks and asymmetric visibility (8.4, 2024), and in 8.5 the pipe
operator `|>`, a standards-compliant URI extension, `clone(...)` with property
updates, `#[\NoDiscard]`, `array_first`/`array_last` and backtraces on fatal
errors [5]; 8.6 (November 2026) adds partial function application and
`clamp()` [6]. The RFC process (wiki.php.net, public votes by core
contributors, 2/3 supermajority since the "Abolish Narrow Margins" RFC passed
30-2 in February 2019 [7]) is slow but has let PHP fix real defects: "Saner
string to number comparisons" (8.0) finally made `0 == "foo"` false; the
`@` operator, `extract`, `register_globals` (removed 5.4) and magic quotes
(removed 5.4) are gone. PHP Foundation (2021, funded by JetBrains, Automattic,
Laravel and others) pays several core developers. Around the language,
Composer (2012) and Packagist copied Bundler's manifest/lockfile model,
Laravel and Symfony brought dependency injection and testing culture, and
PHPStan/Psalm provide TypeScript-style gradual analysis; modern PHP code is
closer to Java than to the PHP of 2005.

## Weaknesses

### Perl: TIMTOWTDI versus readability

Context sensitivity (a function returns a list or a count depending on the
caller), sigils that change with access (`@a` vs `$a[0]` vs `$#a`), `local`
versus `my`, implicit `$_`, prototypes, `wantarray`, bareword filehandles,
and the ability to omit parentheses and quotes let the same program be written
in dozens of styles. The "write-only" reputation is the direct result; Python's
"one obvious way" (PEP 20) is explicitly a reaction. The fixes: `use strict`
and `use warnings` (opt-in since the 1990s; `use v5.12` enables strict and
`use v5.36` also enables warnings and signatures), Perl::Critic, Modern Perl
conventions, and in 5.38 a `class` feature (Corinna) replacing `bless`-based
OO after 29 years.

### Perl: the Perl 6 / Raku split that killed momentum

Wall announced Perl 6 on 19 July 2000 as a community redesign ("fix the
language rather than fix the user"); 361 RFCs were collected; Apocalypses and
Synopses followed; implementations on Parrot and Pugs (Audrey Tang, Haskell,
2005) came and went; the first production release (Rakudo on MoarVM) was
25 December 2015, fifteen and a half years later; in October 2019 it was
renamed Raku to free Perl 5 from the "obsolete version" stigma [8]. During
those years Perl 5 was "the old version of a language whose new version was
always coming", CPAN authors did not know which language to target, and
Python and Ruby took the newcomers. The attempted correction was "Perl 7"
(announced 24 June 2020 as Perl 5.32 with modern defaults; revised May 2021
with no timeline; effectively cancelled by the Steering Council over backward
compatibility for scripts not using the pragmas that would become default)
[3]. Perl now releases 5.x yearly with experimental features behind `use
feature` and `use v5.NN` bundles. This is the strongest evidence in this
research that announcing a successor before it exists damages the original.

### Perl: sigils and context as a learnability cost

Learners must grasp that `$x`, `@x`, `%x` and `&x` are four variables, that
`$x[0]` indexes `@x`, that `my ($a) = f()` and `my $a = f()` call `f` in
different contexts, and that references need `->` and `@{...}` (postfix
`->@*` arrived in 5.24, 2016). Raku kept sigils but made them invariant
(`@a[0]`), an admission that the Perl 5 rule was wrong.

### PHP: the inconsistent standard library

Function naming and argument order were never designed: `strlen`,
`str_replace`, `strpos`, `substr_count`, `nl2br`, `htmlspecialchars`,
`ucwords`; `array_map($callback, $array)` but `array_filter($array,
$callback)` and `usort($array, $callback)`; `strpos($haystack, $needle)` but
`in_array($needle, $haystack)`; `array_key_exists($key, $array)` but
`in_array($value, $array)`; underscores present or absent at random. The
2012 essay "PHP: a fractal of bad design" catalogued hundreds of such items
[9]. The core team's own judgement is visible in what has been done since:
"Consistent type errors for internal functions" (8.0), "Saner numeric
strings" (8.0), the `str_contains`/`str_starts_with` additions (8.0) following
modern naming, and the explicit decision (RFC discussions 2015-2020) that
renaming the existing functions would break the web and is not worth it.
Named arguments (8.0) exposed another cost: parameter names became part of the
API overnight, and extension authors had to audit every name.

### PHP: weak typing and coercion by default

`"abc" == 0` was true until PHP 8; `"1e3" == "1000"` is still true; `null`,
`""`, `"0"`, `0`, `[]` are all falsy; string-to-number juggling in arithmetic;
`switch` uses loose comparison; arrays are simultaneously lists and ordered
hash maps with integer-string key normalisation (`"1"` becomes `1`). Declared
types are only enforced strictly with `declare(strict_types=1)` at the top of
each file, and only for calls made from that file, a per-file mode switch that
remains the most confusing part of modern PHP. Gradual typing since 7.0 is
nevertheless widely used in frameworks, and PHPStan level 9 approximates
TypeScript strictness.

### PHP: security history as a design consequence

`register_globals` (request parameters became globals; default on until 4.2,
removed 5.4), magic quotes (removed 5.4), remote `include` by default and
string-concatenated `mysql_query` made PHP the language of SQL-injection
tutorials in the 2000s. Each was removed by RFC, to the process's credit, but
each was a default chosen for "easy things easy" without a threat model.
PHP 8.0's JIT, likewise, helps numeric benchmarks and barely helps typical
I/O-bound requests; opcache remains the practical win, and Perl has no JIT.

## Designer retrospectives

- Larry Wall on Perl 6: "we decided it would be better to fix the language
  than fix the user" [8]; the Apocalypses are a catalogue of Perl 5 regrets
  (sigil variance, context, `bless`, references, no grammars), most acted on
  in Raku and some (postfix deref, `class`, signatures, `say`) back-ported.
- The Steering Council's cancellation of Perl 7 (2021) judged a "clean
  defaults" major version not worth breaking unmodified scripts [3].
- Rasmus Lerdorf has said in many talks ("25 years of PHP", 2019-2020) that
  the inconsistent names came from mirroring the C functions they wrapped and
  that PHP's success came from solving deployment, not from the language.
- The 2019 move to a 2/3 vote and the PHP 8 "saner comparisons" RFC text
  acknowledge that loose comparison was a twenty-year-old design error kept
  for compatibility [7].

## Influence

From Perl: regex syntax (everywhere), CPAN's model (PyPI, RubyGems, npm),
TAP, topic variables (`$_` in Ruby, Raku, PowerShell), heredocs, hash
literals, Ruby's text-processing layer, PHP's early syntax (`$` sigils, `.`
concatenation, associative arrays). From PHP: little language design was
copied, but the shared-nothing deployment model reappeared as serverless, and
Hack (Facebook, 2014) is a typed PHP that in turn shaped PHP 7's declarations.
Into Perl: awk, sed, sh, C, Lisp (closures). Into PHP: Perl (syntax, arrays),
C (library names), Java (PHP 5 object model, exceptions), Hack (types),
Python/Ruby (enums, `match`, named args), JavaScript (`?->`, arrow
functions), F#/Elixir (the pipe operator).

## Lessons for Sisu

- **Adopt**: regex as a first-class, syntax-checked literal with a
  linear-time engine; a central package index with automated cross-platform
  test reporting (CPAN Testers) tied to a lockfile; a "just copy the file"
  deployment path for scripts and handlers (single static binary or script
  mode); a public RFC process with supermajority votes and named release
  managers; a foundation that pays maintainers from the start.
- **Adapt**: PHP shows declared types on a dynamic base get used when
  frameworks require them; Sisu should make strict semantics the only mode,
  not a per-file `strict_types` switch. TIMTOWTDI is right for libraries and
  wrong for core syntax: one canonical form per construct, enforced by a
  formatter. Perl's `use v5.NN` bundles and PHP's deprecate-then-remove
  cadence are both workable edition mechanisms; pick one, bound to the
  manifest.
- **Avoid**: announcing a successor before it runs existing code (Perl 6:
  15 years; Python 3: 12); context-sensitive evaluation and variant sigils;
  a standard library named by accretion (fix naming and argument-order rules
  before the first function, and lint them); loose equality or implicit
  string/number coercion; security-relevant convenience defaults (globals
  from input, remote includes); per-file semantic mode switches.
- **Open questions raised**: whether a large, redundant "natural language"
  vocabulary has any place in a readability-first language or only survives
  in one-liners; whether PHP's 70% web share proves deployment simplicity
  beats language quality, and how a new language reproduces it without
  Apache; whether a 2/3-vote process can make hard decisions (PHP never fixed
  function naming) or only additive ones.

## Sources

1. Stack Overflow Developer Survey 2025, Technology. https://survey.stackoverflow.co/2025/technology (2025)
2. W3Techs, "Usage statistics of server-side programming languages for websites", as reported in secondary coverage, March and June 2026. https://w3techs.com/technologies/overview/programming_language
3. Wikipedia, "Perl" (history, TIMTOWTDI, Perl 7 announcement 24 June 2020 and cancellation, 5.44.0 July 2026). https://en.wikipedia.org/wiki/Perl ; perl5440delta https://perldoc.perl.org/perl5440delta (2026); perl5420delta (2025)
4. PHP 7.0.0 release announcement ("up to twice as fast as PHP 5.6"). https://www.php.net/releases/7_0_0.php (Dec 2015)
5. PHP 8.5 release page and coverage (pipe operator, URI extension, clone with, NoDiscard). https://www.php.net/releases/8.5/en.php (20 Nov 2025)
6. PHP 8.6 release schedule (wiki todo page: beta 13 Aug, hard freeze 22 Sep, GA 19 Nov 2026). https://wiki.php.net/todo/php86 (2026)
7. PHP RFC, "Abolish Narrow Margins" (2/3 supermajority; vote 30-2, Feb 2019). https://wiki.php.net/rfc/abolish-narrow-margins ; PHP RFC "Saner string to number comparisons" https://wiki.php.net/rfc/string_to_number_comparison (2020)
8. Wikipedia, "Raku (programming language)" (announcement 19 July 2000; first release 25 Dec 2015; rename Oct 2019; Wall quote). https://en.wikipedia.org/wiki/Raku_(programming_language)
9. Eevee (Alex Munroe), "PHP: a fractal of bad design". https://eev.ee/blog/2012/04/09/php-a-fractal-of-bad-design/ (2012)
10. Larry Wall, "Perl, the first postmodern computer language" (1999) and "State of the Onion" talks (2000-2015). https://www.perl.com/pub/1999/03/pm.html
11. PHP Foundation announcement. https://opencollective.com/phpfoundation (Nov 2021); Rasmus Lerdorf, "25 Years of PHP" talks (2019-2020)
12. Perl 5.38 perldelta (class feature). https://perldoc.perl.org/perl5380delta (2023); Perl 5.36 (`use v5.36` enabling strict/warnings/signatures) https://perldoc.perl.org/perl5360delta (2022)
