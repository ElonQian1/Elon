package com.elon.acceptance;

import android.graphics.Bitmap;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.pdf.PdfRenderer;
import android.os.ParcelFileDescriptor;
import com.elon.app.WebChatTextBlockPdf;
import java.io.File;
import java.io.FileOutputStream;
import java.nio.charset.StandardCharsets;
import org.json.JSONObject;

/** Runs the compiled production exporter on Android without opening or replacing the app. */
public final class TextBlockPdfAcceptance {
    private static void require(boolean condition, String code) {
        if (!condition) throw new AssertionError(code);
    }

    private static int verify(File directory, String content, boolean wide, boolean empty, boolean preview) throws Exception {
        byte[] bytes = WebChatTextBlockPdf.INSTANCE.bytes(content);
        require(new String(bytes, 0, 5, StandardCharsets.US_ASCII).equals("%PDF-"), "invalid_pdf_header");
        File file = new File(directory, preview ? "sample.pdf" : "check.pdf");
        try (FileOutputStream output = new FileOutputStream(file)) { output.write(bytes); }
        try (ParcelFileDescriptor fd = ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY);
             PdfRenderer pdf = new PdfRenderer(fd)) {
            require(pdf.getPageCount() > 0, "missing_pdf_pages");
            for (int i = 0; i < pdf.getPageCount(); i++) {
                try (PdfRenderer.Page page = pdf.openPage(i)) {
                    require(page.getWidth() == (wide ? 842 : 595), "wrong_page_width");
                    require(page.getHeight() == (wide ? 595 : 842), "wrong_page_height");
                    Bitmap bitmap = Bitmap.createBitmap(page.getWidth(), page.getHeight(), Bitmap.Config.ARGB_8888);
                    try {
                        bitmap.eraseColor(Color.WHITE);
                        page.render(bitmap, null, null, PdfRenderer.Page.RENDER_MODE_FOR_DISPLAY);
                        int ink = 0;
                        for (int y = 42; y < page.getHeight() - 42; y++) {
                            for (int x = 42; x < page.getWidth() - 42; x++) {
                                if (Color.red(bitmap.getPixel(x, y)) < 180) ink++;
                            }
                        }
                        require(empty || ink > 30, "blank_body_page");
                        if (preview && i == 0) {
                            try (FileOutputStream output = new FileOutputStream(new File(directory, "preview.png"))) {
                                require(bitmap.compress(Bitmap.CompressFormat.PNG, 100, output), "preview_failed");
                            }
                        }
                    } finally { bitmap.recycle(); }
                }
            }
            return pdf.getPageCount();
        }
    }

    public static void main(String[] args) throws Exception {
        require(args.length == 1 && args[0].matches("/data/local/tmp/elon-pdf-[a-f0-9]{32}"), "invalid_output_directory");
        File directory = new File(args[0]);
        require(directory.isDirectory(), "missing_output_directory");
        // app_process is not forked from the preloaded app zygote. Never do this in the APK.
        if (Typeface.DEFAULT == null) {
            Typeface.class.getDeclaredMethod("loadPreinstalledSystemFontMap").invoke(null);
        }
        require(Typeface.DEFAULT != null, "system_fonts_not_initialized");
        StringBuilder document = new StringBuilder("# Export fixture\n\n**Bold**, *italic*, ~~removed~~ and \u4e2d\u6587.\n\n" +
            "3. First item\n4. Second item\n\n> A quoted paragraph.\n\n" +
            "```python\n    value = '\u4e2d\u6587'\n\n    return value\n```\n\n" +
            "| Column A | Column B |\n|---|---|\n| One | **Two** |\n\n");
        for (int i = 0; i < 160; i++) document.append("Paragraph ").append(i).append("\n\n");
        int pages = verify(directory, document.toString(), false, false, true);
        require(pages > 1, "long_document_not_paginated");
        String row = "| A | B | C | D | E | F |";
        verify(directory, row + "\n|---|---|---|---|---|---|\n" + row, true, false, false);
        require(verify(directory, "", false, true, false) == 1, "empty_document_page_count");
        StringBuilder tallCell = new StringBuilder("| A | B |\n|---|---|\n| ");
        for (int i = 0; i < 350; i++) tallCell.append("cell ").append(i).append(" ");
        tallCell.append(" | other cell |");
        require(verify(directory, tallCell.toString(), false, false, false) > 1, "tall_cell_not_paginated");
        System.out.println("PDF_NATIVE_RESULT=" + new JSONObject().put("schema", "elon.writing_pdf_native.v1")
            .put("passed", true).put("cases", 4).put("sample_pages", pages).put("preview", true)
            .put("production_ui_verified", false).put("private_data_accessed", false));
    }
}
