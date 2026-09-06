// ROM-free regression against the actual vendored Screen implementation (ISC).
#include <ares/ares.hpp>
#include <thread>
#include <cstdio>
#include <cstdlib>

namespace ares {
Platform* platform = nullptr;
atomic<bool> _runAhead = false;
}
namespace ares::Core::Video {
#include <ares/node/video/sprite.cpp>
#include <ares/node/video/screen.cpp>
}

static void require(bool ok, const char* message) {
  if(!ok) { std::fprintf(stderr, "%s\n", message); std::exit(1); }
}

struct Observer : ares::Platform {
  std::thread::id caller = std::this_thread::get_id();
  unsigned publications = 0;
  unsigned expected = 0;
  auto video(ares::Node::Video::Screen, const u32* data, u32 pitch,
             u32 width, u32 height) -> void override {
    require(std::this_thread::get_id() == caller, "publication escaped caller thread");
    require(width == 512 && height == 480 && pitch == 512 * 4, "unexpected extent");
    for(unsigned y = 0; y < height; ++y) {
      constexpr u32 colors[] = {0xff000000, 0xffff0000, 0xff00ff00, 0xff0000ff};
      for(unsigned x = 0; x < width; ++x)
        require(data[y * (pitch / 4) + x] == colors[(x + 3 * y + expected) % 4], "incomplete/wrong epoch");
      // Deliberately yield during publication, not a completion sleep/wait.
      if(y % 32 == 0) std::this_thread::yield();
    }
    ++publications;
  }
};

int main() {
  require(!ares::Video::Threaded, "oracle requires synchronous completed-frame publication");
  Observer observer;
  ares::platform = &observer;
  // Exercise both explicit quit + destruction and direct destruction.
  for(unsigned lifecycle = 0; lifecycle < 4; ++lifecycle) {
    auto screen = std::make_shared<ares::Core::Video::Screen>("Screen", 512, 480);
    screen->setViewport(0, 0, 512, 480);
    screen->colors(4, [](n32 value) -> n64 {
      constexpr uint64_t colors[] = {0, 0xffff00000000ull, 0x0000ffff0000ull, 0x00000000ffffull};
      return colors[value];
    });
    screen->power();
    unsigned refreshes = 0;
    screen->setRefresh([&] { ++refreshes; });
    for(unsigned frame = 0; frame < 128; ++frame) {
      observer.expected = frame;
      auto pixels = screen->pixels();
      for(unsigned y = 0; y < 480; ++y)
        for(unsigned x = 0; x < 512; ++x) pixels[y * 512 + x] = (x + 3 * y + frame) % 4;
      auto before = observer.publications;
      screen->frame();
      require(observer.publications == before + 1, "frame returned before publication");
      require(refreshes == frame + 1, "refresh callback not completed");
    }
    if(lifecycle % 2 == 0) { screen->quit(); screen->quit(); }
  }
  ares::platform = nullptr;
  std::puts("512 full-frame publications + quit/destructor lifecycles passed");
}
