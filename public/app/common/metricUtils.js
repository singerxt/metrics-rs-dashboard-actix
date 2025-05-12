/**
 * Groups metrics data by label type to create a format compatible with ApexCharts.
 *
 * @description
 * This function transforms an array of metric objects into a format suitable for
 * rendering multiple series in ApexCharts. Each series represents a different label type.
 *
 * @param {Array<Object>} metrics - An array of metric objects, each containing timestamp, value, and labels properties.
 * @returns {Array<Object>} An array of series objects, each with a name (the label type) and data points (timestamp and value pairs).
 *
 * @example
 * const metrics = [
 *   { timestamp: "2023-01-01T00:00:00Z", value: 10, labels: { type: "cpu" } },
 *   { timestamp: "2023-01-01T00:01:00Z", value: 15, labels: { type: "memory" } }
 * ];
 * const series = groupByLabelType(metrics);
 * // Returns:
 * // [
 * //   { name: "cpu", data: [{ x: 1672531200000, y: 10 }] },
 * //   { name: "memory", data: [{ x: 1672531260000, y: 15 }] }
 * // ]
 */
function groupByLabelType(metrics) {
  return metrics.reduce((acc, metric) => {
    const label = metric?.labels?.type || "default";
    const current = acc.find((item) => item.name === label);

    if (current) {
      current.data.push({
        x: new Date(metric.timestamp).getTime(),
        y: metric.value,
      });
    } else {
      acc.push({
        name: label,
        data: [
          {
            x: new Date(metric.timestamp).getTime(),
            y: metric.value,
          },
        ],
      });
    }
    return acc;
  }, []);
}

function normalizeFloat(value) {
  if (Number.isInteger(value)) {
    return value;
  }

  return Number.parseFloat(value).toFixed(2);
}

export { groupByLabelType, normalizeFloat };
