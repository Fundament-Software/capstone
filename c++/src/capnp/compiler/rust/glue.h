// glue.h

#pragma once
#include "rust/cxx.h"

rust::Vec<uint8_t> command(rust::Slice<const rust::String> files,
  rust::Slice<const rust::String> imports,
  rust::Slice<const rust::String> prefixes,
  bool standard_import);

uint64_t genRandId();
