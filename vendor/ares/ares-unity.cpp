// Project-authored ares unity source for the headless oracle.
//
// Combines the ares framework (minus the GUI-only nall GDB server), the SFC
// processor components, and the Super Famicom core into one translation
// unit. Built by crates/oracle/build.rs; all sources are vendored under
// vendor/ares/ (ISC; see LICENSE.txt next to this file).
//
// This file is derived from ares's generated translation unit
// (ares/ares.cpp.in + the sfc core), reorganized to exclude the debug
// server and include exactly the pieces the Oracle needs.
#include <ares/ares.hpp>
#include <ares/debug/debug.cpp>
#include <ares/node/node.cpp>
#include <ares/resource/resource.cpp>

namespace ares {
Platform* platform = nullptr;
atomic<bool> _runAhead = false;
const string Name       = "ares";
const string Version    = "vendored oracle build";
const string Copyright  = "2004-2026 ares team, Near";
const string License    = "ISC";
const string LicenseURI = "https://opensource.org/licenses/ISC";
const string Website    = "ares-emu.net";
const string WebsiteURI = "https://ares-emu.net/";
const u32    SerializerSignature = 0x31545342;
}

#include <component/processor/arm7tdmi/arm7tdmi.cpp>
#include <component/processor/gsu/gsu.cpp>
#include <component/processor/hg51b/hg51b.cpp>
#include <component/processor/spc700/spc700.cpp>
#include <component/processor/upd96050/upd96050.cpp>
#include <component/processor/wdc65816/wdc65816.cpp>

#include <sfc/sfc.cpp>
