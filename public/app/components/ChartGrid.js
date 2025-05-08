import {
  html,
  useEffect,
  useRef,
  useState
} from "https://esm.sh/htm/preact/standalone";
import MetricBuffer from "../common/MetricBuffer.js";
import PrometheusImport from "../common/PrometheusImport.js";

const metricBuffer = new MetricBuffer(10);
const prometheusImporter = new PrometheusImport('./prometheus');

function CounterChart({
  metricSample
}) {
  const chartRef = useRef(null);

  useEffect(() => {
    const data = metricSample.metrics.map((metric) => {
      return {
        x: new Date(metric.timestamp).getTime(),
        y: metric.value,
      };
    });

    const options = {
      title: {
        text: metricSample.name,
        align: 'left',
        style: {
          fontSize: '24px',
          color: '#fff'
        }
      },
      subtitle: {
        text: `${metricSample.help} | current: ${data[data.length - 1].y}`,
        align: 'left',
        style: {
          fontSize: '16px',
          color: '#fff'
        }
      },
      chart: {
        type: 'line',
        height: 350,
        animations: {
          enabled: false,
        },
        toolbar: {
          tools: {
            download: true,
            selection: false,
            zoom: false,
            zoomin: false,
            zoomout: false,
            pan: false,
            reset: false,
            customIcons: []
          },
        },
      },
      series: [{
        name: 'Counter Value',
        data: data
      }],
      xaxis: {
        type: 'datetime',
        title: {
          text: 'Time'
        }
      },
      yaxis: {
        title: {
          text: 'Value',
        }
      },
      tooltip: {
        x: {
          format: 'HH:mm:ss:ms'
        }
      },
      theme: {
        enabled: true,
        color: '#255aee',
        shadeTo: 'dark',
        shadeIntensity: 0.65
      }
    };

    const chart = new ApexCharts(chartRef.current, options);
    chart.render();

    return () => {
      chart.destroy();
    };
  }, [JSON.stringify(metricSample)]);

  return html`<div ref=${chartRef}></div>`;
}

const renderChart = (sample) => {
  switch (sample.type) {
    case "COUNTER": {
      return html`<${CounterChart} metricSample=${sample} />`;
    }
    default: {
      return html`<h1>Unsupported metric type: ${sample.type}</h1>`;
    }
  }
};

function ChartGrid({
  searchValue,
  refreshRate,
  bufferSize
}) {
  const [metrics, setMetrics] = useState([]);

  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const metrics = await prometheusImporter.fetchMetrics();
        metricBuffer.addMetrics(metrics);
        const filteredMetrics = metricBuffer.getMetrics().filter((sample) => {
          if (!searchValue) return true;
          return sample.name.toLowerCase().includes(searchValue.toLowerCase());
        });
        setMetrics(filteredMetrics);
      } catch (error) {
        console.error("Error fetching metrics:", error);
        // Optionally display error to user or retry
      }
    }, refreshRate);
    return () => clearInterval(interval);
  }, [refreshRate, searchValue]);

  useEffect(() => {
    metricBuffer.setBufferSize(bufferSize);
  }, [bufferSize]);

  return html`
    <div class="responsive-grid">
      ${metrics.map((sample) => renderChart(sample))}
    </div>
  `;
}

export default ChartGrid;
