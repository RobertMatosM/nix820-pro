# First release preparation

The source is a standalone Git repository under the working name Keyboard Studio. Original code is 0BSD. No GitHub repository, remote, tag or release is created automatically.

Completed: research/inactive UI removed; vendor images removed; original flat drawing, SVG icon, text sidebar and action search; native window decoration/resizing; XDG storage and explicit migration; legacy unknown preferences retained; scoped test fixtures separated from runtime; strict lint checks; CI; optimized stripped packaging; bundled font/dependency notices.

Before publishing: choose/check the permanent project name; review the new UI on a real desktop; test the refactored binary against the keyboard; test the release on intended Linux distributions; confirm the support/firmware matrix and installation instructions. The coding environment's native display cannot be used for screenshots. Headless tests do not replace that visual/physical review.

Build outputs, distributions, raw captures, generated capture output and personal settings are ignored. Release archives contain the binary, documentation/notices, icon/desktop entry and user installer. Original private captures and the prototype remain in the old workspace. The development/test data in fixtures contains only intentional configuration traffic.
