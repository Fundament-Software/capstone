// glue.h

#pragma once
#include "rust/cxx.h"

rust::Vec<uint8_t> command(rust::Slice<const rust::String> files,
                           rust::Slice<const rust::String> imports,
                           rust::Slice<const rust::String> prefixes,
                           rust::Slice<const rust::String> memoryFilesData,
                           rust::Slice<const rust::String> memoryFilesPaths,
                           bool standard_import);

uint64_t genRandId();
