import struct
from unittest import TestCase

import maturin_import_hook

maturin_import_hook.install()

import fast_pdf_extract


class ImageTests(TestCase):
    def test_get_images(self) -> None:
        for filename, count, filtered_dimensions in [
            ("mini-pnb.pdf", 501, [(2532, 3271), (696, 464)]),
            ("i32-box.pdf", 26, [(528, 582)]),
        ]:
            with self.subTest(filename=filename):
                path = f"tests/test_files/{filename}"
                images = fast_pdf_extract.get_images(path)
                # Counts exclude vector gradients and separately stored soft masks.
                self.assertEqual(len(images), count)
                for image in images:
                    self.assertIsInstance(image, bytes)
                    self.assertEqual(image[:8], b"\x89PNG\r\n\x1a\n")

                filtered = fast_pdf_extract.get_images(
                    path, min_long_side=500, min_short_side=300
                )
                # Thresholds use page points; returned PNGs keep native pixel dimensions.
                self.assertEqual(
                    [struct.unpack(">II", image[16:24]) for image in filtered],
                    filtered_dimensions,
                )
                for image in filtered:
                    self.assertIn(image, images)

                # Each limit must work independently when the other is disabled.
                self.assertEqual(
                    fast_pdf_extract.get_images(path, min_long_side=100000), []
                )
                self.assertEqual(
                    fast_pdf_extract.get_images(path, min_short_side=100000), []
                )
