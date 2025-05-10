// Produce metrics group by sample.labels.type compatible with ApexCharts.
// This is used to create a chart with multiple series, each representing a different label type.
function groupByLabelType(metrics) {
  return metrics.reduce((acc, metric) => {
    const label = metric.labels.type;
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

export { groupByLabelType };
