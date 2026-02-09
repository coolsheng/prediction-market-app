import { StatusBar } from 'expo-status-bar';
import { StyleSheet, Text, View, TextInput, TouchableOpacity, ScrollView, KeyboardAvoidingView, Platform, SafeAreaView } from 'react-native';
import { useState } from 'react';

interface PredictionResponse {
  prediction: string;
  confidence: number;
  key_factors: string[];
  risks: string[];
  time_sensitivity: string;
  timestamp: number;
  model: string;
}

export default function App() {
  const [url, setUrl] = useState<string>('');
  const [response, setResponse] = useState<PredictionResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(false);

  const handleSubmit = async () => {
    if (!url.trim()) return;

    setIsLoading(true);
    setResponse(null);
    setError(null);

    try {
      const apiKey = process.env.EXPO_PUBLIC_API_KEY;
      const backendUrl = process.env.EXPO_PUBLIC_BACKEND_URL;

      if (!apiKey) {
        throw new Error('API key is not configured. Please set EXPO_PUBLIC_API_KEY in your .env file.');
      }

      if (!backendUrl) {
        throw new Error('Backend URL is not configured. Please set EXPO_PUBLIC_BACKEND_URL in your .env file.');
      }
      const response = await fetch(`${backendUrl}/lambda-url/bootstrap`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'x-api-key': apiKey
        },
        body: JSON.stringify({ market_url: url.trim() }),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
        throw new Error(errorData.error || `HTTP error! status: ${response.status}`);
      }

      const data: PredictionResponse = await response.json();
      setResponse(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch data from server');
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <SafeAreaView style={styles.container}>
      <KeyboardAvoidingView
        style={styles.keyboardView}
        behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
        keyboardVerticalOffset={Platform.OS === 'ios' ? 0 : 20}
      >
        <StatusBar style="light" />

        {/* Header */}
        <View style={styles.header}>
          <Text style={styles.headerTitle}>Idiot Market Prediction</Text>
        </View>

        {/* Response Display Area */}
        <ScrollView
          style={styles.responseContainer}
          contentContainerStyle={styles.responseContent}
          showsVerticalScrollIndicator={true}
          keyboardShouldPersistTaps="handled"
        >
          {error ? (
            <View style={[styles.messageBubble, styles.errorBubble]}>
              <Text style={styles.errorText}>Error: {error}</Text>
            </View>
          ) : response ? (
            <View style={styles.responseCard}>
              {/* Prediction */}
              <View style={styles.section}>
                <Text style={styles.sectionTitle}>Prediction</Text>
                <Text style={styles.predictionText}>{response.prediction}</Text>
              </View>

              {/* Confidence */}
              <View style={styles.section}>
                <Text style={styles.sectionTitle}>Confidence</Text>
                <View style={styles.confidenceBar}>
                  <View
                    style={[styles.confidenceFill, { width: `${response.confidence * 100}%` }]}
                  />
                </View>
                <Text style={styles.confidenceText}>{(response.confidence * 100).toFixed(0)}%</Text>
              </View>

              {/* Key Factors */}
              <View style={styles.section}>
                <Text style={styles.sectionTitle}>Key Factors</Text>
                {response.key_factors.map((factor, index) => (
                  <View key={index} style={styles.bulletItem}>
                    <Text style={styles.bullet}>•</Text>
                    <Text style={styles.bulletText}>{factor}</Text>
                  </View>
                ))}
              </View>

              {/* Risks */}
              <View style={styles.section}>
                <Text style={styles.sectionTitle}>Risks</Text>
                {response.risks.map((risk, index) => (
                  <View key={index} style={styles.bulletItem}>
                    <Text style={styles.bullet}>•</Text>
                    <Text style={styles.bulletText}>{risk}</Text>
                  </View>
                ))}
              </View>

              {/* Time Sensitivity */}
              <View style={styles.section}>
                <Text style={styles.sectionTitle}>Time Sensitivity</Text>
                <Text style={styles.infoText}>{response.time_sensitivity}</Text>
              </View>

              {/* Metadata */}
              <View style={styles.metadataSection}>
                <Text style={styles.metadataText}>
                  Model: {response.model}
                </Text>
                <Text style={styles.metadataText}>
                  Generated: {new Date(response.timestamp * 1000).toLocaleString()}
                </Text>
              </View>
            </View>
          ) : (
            <View style={styles.placeholderContainer}>
              <Text style={styles.placeholderText}>
                Enter a URL below to get started
              </Text>
            </View>
          )}
        </ScrollView>

        {/* Input Area */}
        <View style={styles.inputContainer}>
          <TextInput
            style={styles.input}
            placeholder="Enter URL here..."
            placeholderTextColor="#8e8ea0"
            value={url}
            onChangeText={setUrl}
            autoCapitalize="none"
            autoCorrect={false}
            keyboardType="url"
            returnKeyType="send"
            onSubmitEditing={handleSubmit}
            editable={!isLoading}
          />
          <TouchableOpacity
            style={[styles.submitButton, (!url.trim() || isLoading) && styles.submitButtonDisabled]}
            onPress={handleSubmit}
            disabled={!url.trim() || isLoading}
            activeOpacity={0.7}
          >
            <Text style={styles.submitButtonText}>
              {isLoading ? '...' : '↑'}
            </Text>
          </TouchableOpacity>
        </View>
      </KeyboardAvoidingView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#343541',
  },
  keyboardView: {
    flex: 1,
  },
  header: {
    paddingTop: Platform.OS === 'ios' ? 10 : 20,
    paddingHorizontal: 20,
    paddingBottom: 20,
    backgroundColor: '#343541',
    borderBottomWidth: 1,
    borderBottomColor: '#4d4d4f',
    alignItems: 'center',
  },
  headerTitle: {
    fontSize: 20,
    fontWeight: '600',
    color: '#ffffff',
  },
  responseContainer: {
    flex: 1,
    backgroundColor: '#343541',
  },
  responseContent: {
    padding: 20,
    paddingBottom: 40,
    flexGrow: 1,
  },
  placeholderContainer: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    minHeight: 200,
  },
  placeholderText: {
    fontSize: 16,
    color: '#8e8ea0',
    textAlign: 'center',
  },
  messageBubble: {
    backgroundColor: '#444654',
    padding: 16,
    borderRadius: 12,
    maxWidth: '100%',
  },
  inputContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: 16,
    paddingVertical: 12,
    paddingBottom: Platform.OS === 'ios' ? 30 : 12,
    backgroundColor: '#40414f',
    borderTopWidth: 1,
    borderTopColor: '#4d4d4f',
  },
  input: {
    flex: 1,
    backgroundColor: '#40414f',
    borderRadius: 8,
    paddingHorizontal: 16,
    paddingVertical: 12,
    fontSize: 16,
    color: '#ffffff',
    borderWidth: 1,
    borderColor: '#4d4d4f',
    maxHeight: 100,
  },
  submitButton: {
    width: 36,
    height: 36,
    borderRadius: 8,
    backgroundColor: '#19c37d',
    justifyContent: 'center',
    alignItems: 'center',
    marginLeft: 12,
  },
  submitButtonDisabled: {
    backgroundColor: '#4d4d4f',
  },
  submitButtonText: {
    color: '#ffffff',
    fontSize: 18,
    fontWeight: '600',
  },
  // Error styles
  errorBubble: {
    backgroundColor: '#5c2b2b',
    borderColor: '#ff4444',
    borderWidth: 1,
  },
  errorText: {
    fontSize: 16,
    color: '#ff6b6b',
    lineHeight: 24,
  },
  // Response card styles
  responseCard: {
    backgroundColor: '#444654',
    borderRadius: 12,
    padding: 16,
  },
  section: {
    marginBottom: 20,
  },
  sectionTitle: {
    fontSize: 14,
    fontWeight: '700',
    color: '#19c37d',
    marginBottom: 8,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  predictionText: {
    fontSize: 16,
    color: '#ffffff',
    lineHeight: 24,
  },
  // Confidence bar styles
  confidenceBar: {
    height: 8,
    backgroundColor: '#4d4d4f',
    borderRadius: 4,
    marginTop: 8,
    marginBottom: 8,
    overflow: 'hidden',
  },
  confidenceFill: {
    height: '100%',
    backgroundColor: '#19c37d',
    borderRadius: 4,
  },
  confidenceText: {
    fontSize: 14,
    color: '#8e8ea0',
    textAlign: 'right',
  },
  // Bullet list styles
  bulletItem: {
    flexDirection: 'row',
    marginBottom: 6,
  },
  bullet: {
    fontSize: 16,
    color: '#19c37d',
    marginRight: 8,
    lineHeight: 22,
  },
  bulletText: {
    fontSize: 15,
    color: '#d1d5db',
    lineHeight: 22,
    flex: 1,
  },
  infoText: {
    fontSize: 15,
    color: '#d1d5db',
    lineHeight: 22,
  },
  // Metadata styles
  metadataSection: {
    marginTop: 16,
    paddingTop: 16,
    borderTopWidth: 1,
    borderTopColor: '#4d4d4f',
  },
  metadataText: {
    fontSize: 12,
    color: '#6b7280',
    marginBottom: 4,
  },
});
