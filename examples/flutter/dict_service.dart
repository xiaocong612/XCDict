// dict_service.dart — Flutter asset loader and bridge integration template.
//
// Add to your pubspec.yaml:
// ```yaml
// dependencies:
//   path_provider: ^2.1.2
//
// flutter:
//   assets:
//     - assets/db/xcdict.db
// ```

import 'dart:io';
import 'package:flutter/services.dart';
import 'package:path_provider/path_provider.dart';

class XCDictLoader {
  static const String assetDbPath = 'assets/db/xcdict.db';
  static const String dbFileName = 'xcdict.db';

  /// Ensures xcdict.db is extracted from assets to the local app documents directory,
  /// returning the absolute file path to pass into Rust or SQLite driver.
  static Future<String> prepareDatabasePath() async {
    final docDir = await getApplicationDocumentsDirectory();
    final dbFile = File('${docDir.path}/$dbFileName');

    // Copy from bundled assets on first run or when version updates
    if (!await dbFile.exists()) {
      final ByteData data = await rootBundle.load(assetDbPath);
      final List<int> bytes = data.buffer.asUint8List(
        data.offsetInBytes,
        data.lengthInBytes,
      );
      await dbFile.writeAsBytes(bytes, flush: true);
    }

    return dbFile.path;
  }
}
