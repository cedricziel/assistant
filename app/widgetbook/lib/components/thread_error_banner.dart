import 'package:assistant_ui/assistant_ui.dart';
import 'package:widgetbook/widgetbook.dart';

final threadErrorBannerComponent = WidgetbookComponent(
  name: 'ThreadErrorBanner',
  useCases: [
    WidgetbookUseCase(
      name: 'default',
      builder: (context) => ThreadErrorBanner(
        message: 'The connection to the server was lost.',
        onDismiss: () {},
      ),
    ),
  ],
);
