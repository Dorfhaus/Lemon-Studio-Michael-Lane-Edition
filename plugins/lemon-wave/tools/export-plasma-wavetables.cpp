// SPDX-License-Identifier: GPL-3.0-only
// Export the canonical frames straight from ML Plasma's vendored factory generator.
#include <cstdint>
#include <cstdlib>
#include <fstream>
#include <limits>
#include "../../ml-plasma/vendor/gridline/src/core/SfzText.h"
#include "../../ml-plasma/vendor/gridline/src/engine/Sample.h"
#include "../../ml-plasma/vendor/gridline/src/engine/pitch/Kernel.h"
#include "../../ml-plasma/vendor/gridline/src/packs/plasma/plasma.h"

// The factory generator shares a translation unit with file import. The exporter
// never imports files. Fail immediately if a future generator starts using these.
namespace gl {
SamplePtr loadSample(const std::string&, std::string*) { std::abort(); }
namespace sfz {
bool expand(const std::string&, Expanded&) { std::abort(); }
}
namespace pitch {
const SincTable& table(Quality) { std::abort(); }
}
}

int main(int argc, char** argv) {
  using namespace gl::packs::plasma;
  if (argc != 2) return 2;
  static_assert(sizeof(float) == 4 && std::numeric_limits<float>::is_iec559);
  const uint16_t endian = 1;
  if (*reinterpret_cast<const char*>(&endian) != 1) return 2;
  std::ofstream file(argv[1], std::ios::binary);
  const auto write32 = [&](uint32_t value) {
    file.write(reinterpret_cast<const char*>(&value), sizeof(value));
  };
  const auto& names = factoryTableNames();
  file.write("PWT1", 4);
  write32(kFrameSize);
  write32(static_cast<uint32_t>(names.size()));
  for (size_t i = 0; i < names.size(); ++i) {
    const auto table = factoryTable(static_cast<int>(i));
    write32(static_cast<uint32_t>(table->name.size()));
    write32(table->frames);
    file.write(table->name.data(), table->name.size());
    file.write(reinterpret_cast<const char*>(table->data.data()), table->data.size() * sizeof(float));
  }
  file.close();
  return file ? 0 : 1;
}
