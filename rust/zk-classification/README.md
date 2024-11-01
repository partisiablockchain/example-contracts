# Secure evaluation of decision tree

The contract allows secret evaluation of a pre-trained model on a user-provided input sample.
The pre-trained model is provided by the model owner, while the input owner provides the sample(s) to be classified.

The model is a decision tree classifier comprising internal vertices and leaf vertices.
Internal vertices consist of a splitting feature and a threshold value, both of which are treated as secrets and secret shared.
Leaf vertices represent the predicted class resulting from following the path from the root to the given leaf. These classes are also secret shared.

**The flow of the contract is as follows:**
1. Initialization of contract.
2. Model and input sample are added.
3. Each internal vertex is evaluated using oblivious lookup in the input sample. Result is a vector of length m of secret-shared bits, where m is the number of internal vertices.
4. Each path through the tree is evaluated through a series of sequential multiplications. Result is a new vector of length n of secret-shared bits, where n is the number of leaf vertices.
5. Each bit is multiplied onto the corresponding class variable, and logical OR is taken of the products. Result is the final output.
6. The final output (predicted class) is given to the input sample owner (or whoever they assign it to) and kept secret from everyone else.


### Example of a secret decision tree classifier
```text
                  (8, 0.5)
                /           \
       (2, 7669.5)           (1, 12.5)
       /        \            /        \
  (1, 12.5)  (0, 20.0)   (2, 5095.5)  (2, 5095.5)
    /   \      /   \        /   \       /   \
   0     0    0     1      0     1     1     1
```

When inputted by the model owner, the model is in JSON format, where vertices are divided into
internal vertices and leaf vertices. Both the internal vertices and the leaf vertices are listed
according to a pre-order traversal of the tree. E.g.:
```text
{ "internals": [ { "feature": 8, "threshold": 0.5 }, ... ], "leaves": [ { "classification": 0 }, ... ] }
```


### Example of a secret input sample
```text
(42, 10, 6000, 0, 37, 0, 1, 0, 1, 1)
```

If the provided input value is less than or equal to the threshold for the given feature, the left path is taken. Otherwise, the right path is taken.


### Example of code to train decision tree classifier

The following Python code represents an example of how one can train a decision tree classifier and save the resulting model in a format suitable for a smart contract. This example uses the scikit-learn library to create and train a model and subsequently saves it in JSON format, but many other approaches exist.

```python
import pandas as pd
import numpy as np
import json
from sklearn.tree import DecisionTreeClassifier, export_text
from sklearn.model_selection import train_test_split


"""
TRAINING THE MODEL
"""
# prepare data
data = pd.read_csv('data/census.csv')
features = data.columns[:-1]
X = data[features]
Y = data['income_>50K']

X_train, X_test, Y_train, Y_test = train_test_split(X.values, Y, test_size=0.3)

# train the model
classifier = DecisionTreeClassifier(criterion='entropy', max_depth=3)
classifier = classifier.fit(X_train, Y_train)
Y_pred = classifier.predict(X_test)


"""
SAVING THE MODEL
"""
text_tree = export_text(classifier, feature_names=features)
print(text_tree)

tree = classifier.tree_
leaf_vertices = []
internal_vertices = []

for i in range(tree.node_count):
  is_leaf = tree.children_left[i] == -1

  if is_leaf:
    pred_class = int(np.argmax(tree.value[i]))
    leaf_vertices.append({'classification': pred_class})
  else:
    feature_index = int(tree.feature[i])
    threshold = float(tree.threshold[i])
    internal_vertices.append({'feature': feature_index, 'threshold': threshold})

vertices = { 'internals': internal_vertices, 'leaves': leaf_vertices }

# save to file
outfile = open('output/model.json', 'w')
json.dump(vertices, outfile)
```

### Example of code to preprocess dataset

The following Python code represents an example of how one can preprocess a dataset to prepare it for model training. The preprocessing choices made in the code reflect a need to balance computational complexity and accuracy. Many other choices could have been made for equally valid reasons.

```python
import pandas as pd
import numpy as np
from sklearn.preprocessing import OneHotEncoder

# read data
data = pd.read_csv('data/adult.csv')

# remove all rows with missing values in any column
data = data[~data.isin(["?"]).any(axis=1)]

# drop features with low importance
data = data.drop(['fnlwgt', 'education', 'occupation', 'relationship', 'race', 'native.country'], axis=1)

# changing marital status to binary (married/not married)
married = ['Married-civ-spouse', 'Separated', 'Married-spouse-absent', 'Married-AF-spouse']
data['marital.status'] = np.where(data['marital.status'].isin(married), 'Married', 'Not-married')

# encode categorical data into numerical data (one-hot encoded vectors)
categorical_cols = ['workclass', 'marital.status', 'sex', 'income']
encoder = OneHotEncoder(sparse_output=False, dtype=int)
encoder.set_output(transform='pandas')
data_encoded = encoder.fit_transform(data[categorical_cols])

# remove columns with less than 50 examples (infrequent categorical values)
data_encoded = data_encoded.loc[:, data_encoded.sum() >= 50]

# clean features by combining similar columns
gov_jobs = ['workclass_Federal-gov', 'workclass_Local-gov', 'workclass_State-gov']
data_encoded.insert(1, 'employment_Gov', data_encoded[gov_jobs].max(axis=1))
data_encoded = data_encoded.drop(gov_jobs, axis=1)

self_emp = ['workclass_Self-emp-inc', 'workclass_Self-emp-not-inc']
data_encoded.insert(2, 'employment_Self', data_encoded[self_emp].max(axis=1))
data_encoded = data_encoded.drop(self_emp, axis=1)

# drop superfluous columns (binary complement exists)
data_encoded = data_encoded.drop(columns=['marital.status_Not-married', 'sex_Male', 'income_<=50K'], axis=1)

# remove non-encoded columns and replace with encoded versions
data = data.drop(columns=categorical_cols)
data = pd.concat([data, data_encoded], axis=1)

# rename features
data = data.rename(columns={
    'education.num': 'education-level', 
    'capital.gain': 'capital-gain',
    'capital.loss': 'capital-loss',
    'hours.per.week': 'hours-per-week',
    'workclass_Private': 'employment_Private',
    'marital.status_Married': 'married',
    'sex_Female': 'female'
})


# save cleaned dataset to file
data.to_csv('data/census.csv', index=False)
```