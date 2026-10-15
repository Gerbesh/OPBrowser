# OPBrowser real-web compatibility baseline

Run: 2026-10-15T10:37:05.920516+00:00. Original Rust engine, not Chromium.
Timeout per site: 23.0s.
These are live changing sites. A display list is not proof of web compatibility.

| Site | Outcome | Load ms | JS failed | Paint commands | Screenshot |
| --- | --- | ---: | ---: | ---: | --- |
| google | rendered_with_js_errors | 381 | 2 | 6 | [Image](google.png) |
| youtube | no_text_rendered | 1667 | 12 | 130 | [Image](youtube.png) |
| reddit | blank_viewport | 248 | 1 | 1 | [Image](reddit.png) |
| deviantart | rendered_with_js_errors | 10117 | 4 | 1731 | [Image](deviantart.png) |
| stackoverflow | navigation_error | 238 | - | - | - |
| twitch | no_text_rendered | 583 | 13 | 9 | [Image](twitch.png) |
| pikabu | rendered_with_js_errors | 14754 | 14 | 1392 | [Image](pikabu.png) |
| instagram | navigation_error | 544 | - | - | - |
| github_explore | rendered_unverified | 13224 | 0 | 1740 | [Image](github_explore.png) |
| tiktok | blank_viewport | 543 | 16 | 2 | [Image](tiktok.png) |
| x_explore | blank_viewport | 3083 | 9 | 1 | [Image](x_explore.png) |
| canva | navigation_error | 331 | - | - | - |
| figma | rendered_with_js_errors | 2759 | 4 | 964 | [Image](figma.png) |
| notion | rendered_with_js_errors | 4949 | 5 | 164 | [Image](notion.png) |
| habr | rendered_with_js_errors | 6125 | 10 | 466 | [Image](habr.png) |
| wikipedia | rendered_with_js_errors | 2801 | 2 | 7046 | [Image](wikipedia.png) |
| mdn | rendered_with_js_errors | 1398 | 1 | 842 | [Image](mdn.png) |
| hackernews | rendered_with_js_errors | 1535 | 1 | 159 | [Image](hackernews.png) |
| amazon | rendered_with_js_errors | 3402 | 8 | 303 | [Image](amazon.png) |
| discord | timeout | - | - | - | - |

## Limits

blank_viewport: white or effectively empty output, even if navigation succeeded.
no_text_rendered: only skeleton/decorative paint commands; no text primitives.
rendered_with_js_errors: page has pixels, but one or more JS callbacks failed.
rendered_unverified: does not imply a correct layout, interactivity, or JS support.
A rendered page may be incorrect or non-interactive; comparison with a reference browser is still needed.
HTTP blocks, bot challenges and geo failures are recorded, not bypassed.
No credentials, authenticated cookies, anti-bot evasion or raw HTML archives.
Detailed metrics and error messages: results.json.
