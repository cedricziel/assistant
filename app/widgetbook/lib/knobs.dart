import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

/// A dropdown knob over every value of an enum, for example `EntryState` or
/// `TimelineDensity`. Pass the enum's `values`.
T enumKnob<T extends Enum>(
  BuildContext context, {
  required String label,
  required List<T> values,
  T? initial,
}) => context.knobs.object.dropdown<T>(
  label: label,
  options: values,
  initialOption: initial ?? values.first,
  labelBuilder: (value) => value.name,
);
