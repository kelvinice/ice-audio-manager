# D-24 · Codex's vote

## Scores

| Option | Score | One-line reason |
|---|---|---|
| G0 · Stay MIT | +1 | The planned shell, Store package and direct downloads can ship under MIT, and the proposed UI and lyrics lookup have independent implementation paths. |
| G1 · GPL-3.0-or-later now | −1 | Each distributed binary would acquire source-delivery obligations before we have chosen a component worth importing. |
| G2 · Revisit for a named component | +2 | An actual component lets the owner weigh the port and its long-term maintenance against an independent Windows implementation. |

## Strongest argument against my top choice

Waiting can make a later transition harder. More contributors and bundled assets mean more rights to trace, and downstream users may already depend on receiving future releases under MIT. If reciprocal licensing is itself the owner's goal, adopting it early gives contributors a clear expectation.

## New option or amendment

For G2, record the upstream revision, exact licence grant and files proposed for reuse in the decision to import. A GPL-3.0-only component can constrain distribution of the combination to version 3 even when our own additions allow later versions. Include the ongoing cost of keeping the port current in the comparison. Previously distributed MIT copies retain their MIT permissions.

## Evidence I checked myself

- **D-09, Microsoft Store.** Yes, the Windows Store distributes GPLv3 apps. Microsoft's developer agreement explicitly assigns FOSS licence compliance and source availability to the publisher. GIMP identifies its licence as GPLv3+ and publishes an official Windows Store package. For our MSIX, present GPL terms in the listing, preserve redistribution rights and provide the matching source link beside the download. Certification is still a separate decision for our app. [Microsoft App Developer Agreement](https://cdn-dynmedia-1.microsoft.com/is/content/microsoftcorp/microsoft/store/documents/legal/ada/fy26/MS.Store.ADAv8.10.EN.US.pdf) · [GIMP licence FAQ](https://www.gimp.org/docs/userfaq.html) · [GIMP Store release](https://www.gimp.org/news/2022/06/18/gimp-2-10-32-on-microsoft-store/)
- **D-09, SignPath Foundation.** Yes, GPLv3 meets its OSI licence condition. The programme also requires active maintenance, excludes commercial dual licensing, and requires a previous release in the form to be signed. The public repo date alone cannot establish NSIS or portable eligibility. Keep the Store-first sequence and confirm those artifact forms with SignPath before promising signed direct downloads. [SignPath eligibility and signing conditions](https://signpath.org/terms) · [OSI approval of GPLv3](https://opensource.org/license/gpl-3.0)
- **D-10, source delivery.** For downloadable MSIX, NSIS and portable ZIP releases I would use GPLv3 §6(d). Publish the exact Corresponding Source at no extra charge, with clear directions beside each binary download, including the Store listing. A separate source host is allowed. Keep the source available for as long as required for those downloads. Include Rust and frontend source, covered dependency source, interface files and scripts needed to build and install that version, including the packaging inputs. System Libraries and general-purpose tools have the §1 exclusions. I verified that the project tracks both lockfiles. A tag and lockfiles alone do not establish completeness. The written-offer route in §6(b) concerns physical products or media and has its own three-year minimum. A written promise is unnecessary when delivering online under §6(d). [GPLv3 §§1 and 6](https://opensource.org/license/gpl-3.0)
- **D-10, notices.** Put the full GPL text, applicable copyright and warranty notices, and required third-party licence texts in each installed package and portable archive. Preserve upstream notices, identify modified covered work and relevant dates in source, and retain applicable interactive legal notices. Give About a version-specific source link as a convenience as well as the directions at download. Signing can continue through the existing channels. Recipients keep the right to build and distribute their own copies. [GPLv3 §§4–6 and 10](https://opensource.org/license/gpl-3.0)

## Position

Keep MIT pending a concrete reuse proposal. Both release channels can accommodate GPLv3. The material difference for release engineering is keeping each shipped binary paired with complete source and notices, including when Store and GitHub versions diverge.
