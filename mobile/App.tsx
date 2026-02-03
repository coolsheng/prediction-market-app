import { StatusBar } from 'expo-status-bar';
import { StyleSheet, Text, View, TouchableOpacity } from 'react-native';
import { useState } from 'react';

export default function App() {
  const [prediction, setPrediction] = useState<string | null>(null);

  const makePrediction = () => {
    // Placeholder for backend integration
    const predictions = [
      "The market will go up 📈",
      "The market will go down 📉",
      "The market will stay stable 📊"
    ];
    const randomPrediction = predictions[Math.floor(Math.random() * predictions.length)];
    setPrediction(randomPrediction);
  };

  return (
    <View style={styles.container}>
      <Text style={styles.title}>Prediction Market App</Text>
      <Text style={styles.subtitle}>Get your market predictions</Text>
      
      <TouchableOpacity style={styles.button} onPress={makePrediction}>
        <Text style={styles.buttonText}>Make Prediction</Text>
      </TouchableOpacity>

      {prediction && (
        <View style={styles.predictionContainer}>
          <Text style={styles.predictionText}>{prediction}</Text>
        </View>
      )}

      <StatusBar style="auto" />
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#fff',
    alignItems: 'center',
    justifyContent: 'center',
    padding: 20,
  },
  title: {
    fontSize: 32,
    fontWeight: 'bold',
    marginBottom: 10,
  },
  subtitle: {
    fontSize: 18,
    color: '#666',
    marginBottom: 40,
  },
  button: {
    backgroundColor: '#007AFF',
    paddingHorizontal: 40,
    paddingVertical: 15,
    borderRadius: 10,
    marginBottom: 20,
  },
  buttonText: {
    color: '#fff',
    fontSize: 18,
    fontWeight: '600',
  },
  predictionContainer: {
    backgroundColor: '#f0f0f0',
    padding: 20,
    borderRadius: 10,
    marginTop: 20,
  },
  predictionText: {
    fontSize: 24,
    textAlign: 'center',
  },
});
